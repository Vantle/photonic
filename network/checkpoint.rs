use crate::configuration::Configuration;
use crate::model::Model;
use crate::optimizer::{Optimizer, Setting};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Failure {
    #[error("could not access the checkpoint: {0}")]
    Access(#[from] std::io::Error),
    #[error("the checkpoint header is malformed: {0}")]
    Header(#[from] serde_json::Error),
    #[error("the checkpoint holds {found} parameters; its configuration needs {expected}")]
    Shape { expected: usize, found: usize },
    #[error("the checkpoint's {head} attention heads do not divide its width of {width}")]
    Head { width: usize, head: usize },
    #[error("the checkpoint's judge has {0} outputs; a judge has 0 or 2")]
    Judge(usize),
    #[error("the checkpoint holds {found} bytes of parameters; its header promises {expected}")]
    Length { expected: u64, found: u64 },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Header {
    configuration: Configuration,
    setting: Setting,
    step: u64,
    parameter: usize,
}

fn write(writer: &mut impl Write, value: &[f32]) -> std::io::Result<()> {
    for value in value {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn read(reader: &mut impl Read, count: usize) -> std::io::Result<Vec<f32>> {
    let mut byte = vec![0u8; count * 4];
    reader.read_exact(&mut byte)?;
    Ok(byte
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect())
}

pub fn save(path: &Path, model: &Model, optimizer: &Optimizer) -> Result<(), Failure> {
    let temporary = path.with_extension("partial");
    {
        let mut writer = BufWriter::new(std::fs::File::create(&temporary)?);
        let header = Header {
            configuration: model.configuration().clone(),
            setting: optimizer.setting,
            step: optimizer.step,
            parameter: model.size(),
        };
        serde_json::to_writer(&mut writer, &header)?;
        writer.write_all(b"\n")?;
        write(&mut writer, &model.parameter)?;
        write(&mut writer, &optimizer.moment)?;
        write(&mut writer, &optimizer.velocity)?;
        writer.flush()?;
    }
    std::fs::rename(&temporary, path)?;
    Ok(())
}

fn check(configuration: &Configuration) -> Result<(), Failure> {
    let (width, head) = (configuration.width, configuration.head);
    if head == 0 || width % head != 0 {
        return Err(Failure::Head { width, head });
    }
    if !matches!(configuration.judge, 0 | 2) {
        return Err(Failure::Judge(configuration.judge));
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<(Model, Optimizer), Failure> {
    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let header: Header = serde_json::from_str(&line)?;
    check(&header.configuration)?;
    let found = size.saturating_sub(line.len() as u64);
    let expected = u64::try_from(header.parameter)
        .ok()
        .and_then(|parameter| parameter.checked_mul(12))
        .unwrap_or(u64::MAX);
    if found != expected {
        return Err(Failure::Length { expected, found });
    }
    let expected = Model::length(&header.configuration);
    if expected != header.parameter {
        return Err(Failure::Shape {
            expected,
            found: header.parameter,
        });
    }
    let parameter = read(&mut reader, header.parameter)?;
    let moment = read(&mut reader, header.parameter)?;
    let velocity = read(&mut reader, header.parameter)?;
    let model = Model::restore(header.configuration, parameter).ok_or(Failure::Shape {
        expected,
        found: header.parameter,
    })?;
    Ok((
        model,
        Optimizer {
            setting: header.setting,
            moment,
            velocity,
            step: header.step,
        },
    ))
}
