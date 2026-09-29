use crate::device::Device;
use crate::operand::{Operand, Product};
use random::Generator;

fn product(left: &[f32], right: &[f32], shape: [usize; 3], transpose: [bool; 2]) -> Vec<f32> {
    let [row, column, interior] = shape;
    let get = |value: &[f32], transposed: bool, index: [usize; 2], stride: [usize; 2]| {
        if transposed {
            value[index[1] * stride[1] + index[0]]
        } else {
            value[index[0] * stride[0] + index[1]]
        }
    };
    let mut result = vec![0.0; row * column];
    for down in 0..row {
        for across in 0..column {
            result[down * column + across] = (0..interior)
                .map(|inner| {
                    get(left, transpose[0], [down, inner], [interior, row])
                        * get(right, transpose[1], [inner, across], [column, interior])
                })
                .sum();
        }
    }
    result
}

#[test]
fn multiply() {
    let Ok(device) = Device::open() else {
        return;
    };
    let mut generator = Generator::new(1);
    for transpose in [[false, false], [true, false], [false, true], [true, true]] {
        let (row, column, interior) = (37, 19, 23);
        let left = (0..row * interior)
            .map(|_| generator.normal() as f32)
            .collect::<Vec<_>>();
        let right = (0..interior * column)
            .map(|_| generator.normal() as f32)
            .collect::<Vec<_>>();
        let offset = 5;
        let mut first = device.memory::<f32>(offset + left.len()).unwrap();
        first.edit::<f32>()[offset..offset + left.len()].copy_from_slice(&left);
        let mut second = device.memory::<f32>(right.len()).unwrap();
        second.edit::<f32>().copy_from_slice(&right);
        let mut third = device.memory::<f32>(row * column).unwrap();
        third.edit::<f32>().fill(1.0);
        let mut command = device.command().unwrap();
        let shape = |transposed: bool, row: usize, column: usize| {
            if transposed {
                (column, row)
            } else {
                (row, column)
            }
        };
        let (left_row, left_column) = shape(transpose[0], row, interior);
        let (right_row, right_column) = shape(transpose[1], interior, column);
        command
            .multiply(Product {
                left: Operand {
                    memory: &first,
                    offset,
                    row: left_row,
                    column: left_column,
                    transpose: transpose[0],
                },
                right: Operand {
                    memory: &second,
                    offset: 0,
                    row: right_row,
                    column: right_column,
                    transpose: transpose[1],
                },
                result: Operand {
                    memory: &third,
                    offset: 0,
                    row,
                    column,
                    transpose: false,
                },
                alpha: 2.0,
                beta: 1.0,
            })
            .unwrap();
        command.run().unwrap();
        let expected = product(&left, &right, [row, column, interior], transpose);
        for (actual, expected) in third.view::<f32>().iter().zip(&expected) {
            assert!(
                (actual - (2.0 * expected + 1.0)).abs() < 1e-3,
                "{transpose:?}"
            );
        }
    }
}

#[test]
fn memory() {
    let Ok(device) = Device::open() else {
        return;
    };
    assert!(!device.name().is_empty());
    let mut memory = device.memory::<u32>(16).unwrap();
    assert!(memory.view::<u32>().iter().all(|&value| value == 0));
    memory.edit::<f32>()[3] = 2.5;
    assert_eq!(memory.view::<f32>()[3], 2.5);
    assert!(device.library("kernel void broken(").is_err());
}

#[test]
fn copy() {
    let Ok(device) = Device::open() else {
        return;
    };
    let value = (0..1000u64).map(|index| index * index).collect::<Vec<_>>();
    let source = device.upload(&value).unwrap();
    let mut target = device.space::<u64>(1200).unwrap();
    target.edit::<u64>().fill(7);
    let mut command = device.command().unwrap();
    command.copy::<u64>(&source, &target, 600).unwrap();
    command.run().unwrap();
    let copied = target.view::<u64>();
    assert_eq!(&copied[..600], &value[..600]);
    assert!(copied[600..].iter().all(|&item| item == 7));
    let mut command = device.command().unwrap();
    assert!(command.copy::<u64>(&source, &target, 1001).is_err());
}

#[test]
fn clear() {
    let Ok(device) = Device::open() else {
        return;
    };
    let mut memory = device.space::<u32>(4099).unwrap();
    memory.edit::<u32>().fill(u32::MAX);
    let mut command = device.command().unwrap();
    command.clear(&memory).unwrap();
    command.run().unwrap();
    assert!(memory.view::<u32>().iter().all(|&item| item == 0));
}

#[test]
fn reach() {
    let Ok(device) = Device::open() else {
        return;
    };
    let source = "
        #include <metal_stdlib>
        using namespace metal;
        struct Table { device uint* part[2]; };
        kernel void gather(device const Table& table [[buffer(0)]], device uint* out [[buffer(1)]], uint index [[thread_position_in_grid]]) {
            out[index] = table.part[index % 2][index / 2];
            table.part[index % 2][8 + index / 2] = index;
        }
    ";
    let library = device.library(source).unwrap();
    let kernel = device.kernel(&library, "gather").unwrap();
    let first = device.upload(&(0..16u32).collect::<Vec<_>>()).unwrap();
    let second = device.upload(&(100..116u32).collect::<Vec<_>>()).unwrap();
    let table = device.upload(&[first.address(), second.address()]).unwrap();
    let mut out = device.memory::<u32>(16).unwrap();
    let mut command = device.command().unwrap();
    command.reach([&first, &second]);
    command
        .dispatch(&kernel, &[&table, &out], &[], [16, 1, 1], [16, 1, 1])
        .unwrap();
    command.run().unwrap();
    let expected = (0..16u32)
        .map(|index| {
            if index % 2 == 0 {
                index / 2
            } else {
                100 + index / 2
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(out.view::<u32>(), expected.as_slice());
    let mut first = first;
    assert_eq!(&first.view::<u32>()[8..16], &[0, 2, 4, 6, 8, 10, 12, 14]);
}
