use ndarray::linalg::general_mat_mul;
use ndarray::{Array2, ArrayView2, ArrayViewMut2, Axis, Slice};
use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub row: usize,
    pub column: usize,
}

impl Span {
    pub fn end(&self) -> usize {
        self.start + self.row * self.column
    }

    pub fn view<'data>(&self, data: &'data [f32]) -> ArrayView2<'data, f32> {
        ArrayView2::from_shape((self.row, self.column), &data[self.start..self.end()])
            .expect("a span addresses a contiguous matrix")
    }

    pub fn edit<'data>(&self, data: &'data mut [f32]) -> ArrayViewMut2<'data, f32> {
        let end = self.end();
        ArrayViewMut2::from_shape((self.row, self.column), &mut data[self.start..end])
            .expect("a span addresses a contiguous matrix")
    }

    pub fn read<'data>(&self, data: &'data [f32]) -> &'data [f32] {
        &data[self.start..self.end()]
    }

    pub fn write<'data>(&self, data: &'data mut [f32]) -> &'data mut [f32] {
        let end = self.end();
        &mut data[self.start..end]
    }
}

pub fn product(left: &ArrayView2<'_, f32>, right: &ArrayView2<'_, f32>) -> Array2<f32> {
    let mut result = Array2::zeros((left.nrows(), right.ncols()));
    general_mat_mul(1.0, left, right, 0.0, &mut result);
    result
}

pub fn accumulate(
    target: &mut ArrayViewMut2<'_, f32>,
    left: &ArrayView2<'_, f32>,
    right: &ArrayView2<'_, f32>,
) {
    general_mat_mul(1.0, left, right, 1.0, target);
}

pub fn view(matrix: &Array2<f32>, row: Range<usize>, column: Range<usize>) -> ArrayView2<'_, f32> {
    matrix
        .slice_axis(Axis(0), Slice::from(row))
        .slice_axis_move(Axis(1), Slice::from(column))
}

pub fn edit(
    matrix: &mut Array2<f32>,
    row: Range<usize>,
    column: Range<usize>,
) -> ArrayViewMut2<'_, f32> {
    matrix
        .slice_axis_mut(Axis(0), Slice::from(row))
        .slice_axis_move(Axis(1), Slice::from(column))
}
