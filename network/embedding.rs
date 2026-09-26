use crate::layout::{Kind, Layout};
use crate::matrix::Span;
use ndarray::{Array2, ArrayView2};
use random::Generator;

#[derive(Clone, Debug)]
pub struct Embedding {
    pub table: Vec<Span>,
}

impl Embedding {
    pub(crate) fn new(layout: &mut Layout, field: &[usize], width: usize) -> Self {
        Self {
            table: field
                .iter()
                .map(|&cardinality| layout.allocate(cardinality, width, Kind::Table))
                .collect(),
        }
    }

    pub(crate) fn initialize(
        &self,
        parameter: &mut [f32],
        generator: &mut Generator,
        deviation: f32,
    ) {
        for table in &self.table {
            for value in table.write(parameter) {
                *value = generator.normal() as f32 * deviation;
            }
        }
    }

    pub(crate) fn forward(&self, parameter: &[f32], feature: &[u16], width: usize) -> Array2<f32> {
        let field = self.table.len();
        let length = feature.len() / field;
        let mut result = Array2::zeros((length, width));
        for (token, mut row) in result.rows_mut().into_iter().enumerate() {
            for (index, table) in self.table.iter().enumerate() {
                let value = usize::from(feature[token * field + index]);
                row += &table.view(parameter).row(value);
            }
        }
        result
    }

    pub(crate) fn backward(
        &self,
        gradient: &mut [f32],
        feature: &[u16],
        delta: &ArrayView2<'_, f32>,
    ) {
        let field = self.table.len();
        for (index, table) in self.table.iter().enumerate() {
            let mut view = table.edit(gradient);
            for (token, row) in delta.rows().into_iter().enumerate() {
                let value = usize::from(feature[token * field + index]);
                view.row_mut(value).scaled_add(1.0, &row);
            }
        }
    }
}
