#include <metal_stdlib>
using namespace metal;

float tangent(float value) {
    if (fabs(value) >= 3.0f) {
        return value > 0.0f ? 1.0f : -1.0f;
    }
    float square = value * value;
    return value * (27.0f + square) / (27.0f + 9.0f * square);
}

float turn(float value) {
    if (fabs(value) >= 3.0f) {
        return 0.0f;
    }
    float square = value * value;
    float denominator = 27.0f + 9.0f * square;
    return ((27.0f + 3.0f * square) * denominator - value * (27.0f + square) * 18.0f * value)
        / (denominator * denominator);
}

float gelu(float value) {
    return 0.5f * value * (1.0f + tangent(0.7978846f * (value + 0.044715f * value * value * value)));
}

float incline(float value) {
    float inner = 0.7978846f * (value + 0.044715f * value * value * value);
    return 0.5f * (1.0f + tangent(inner))
        + 0.5f * value * turn(inner) * 0.7978846f * (1.0f + 3.0f * 0.044715f * value * value);
}

kernel void embed(
    device const uint* feature [[buffer(0)]],
    device const float* parameter [[buffer(1)]],
    device const uint* table [[buffer(2)]],
    device float* output [[buffer(3)]],
    constant uint* shape [[buffer(4)]],
    uint2 position [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], field = shape[2];
    uint row = position.y, column = position.x;
    if (row >= count || column >= width) {
        return;
    }
    float total = 0.0f;
    for (uint index = 0; index < field; index++) {
        total += parameter[table[index] + feature[row * field + index] * width + column];
    }
    output[row * width + column] = total;
}

kernel void norm(
    device const float* input [[buffer(0)]],
    device const float* parameter [[buffer(1)]],
    device float* output [[buffer(2)]],
    device float* normal [[buffer(3)]],
    device float* inverse [[buffer(4)]],
    constant uint* shape [[buffer(5)]],
    uint row [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], scale = shape[2], shift = shape[3], store = shape[4];
    if (row >= count) {
        return;
    }
    device const float* source = input + row * width;
    float mean = 0.0f;
    for (uint column = 0; column < width; column++) {
        mean += source[column];
    }
    mean /= float(width);
    float variance = 0.0f;
    for (uint column = 0; column < width; column++) {
        float difference = source[column] - mean;
        variance += difference * difference;
    }
    variance /= float(width);
    float factor = 1.0f / sqrt(variance + 1e-5f);
    for (uint column = 0; column < width; column++) {
        float value = (source[column] - mean) * factor;
        if (store != 0) {
            normal[row * width + column] = value;
        }
        output[row * width + column] = value * parameter[scale + column] + parameter[shift + column];
    }
    if (store != 0) {
        inverse[row] = factor;
    }
}

kernel void bias(
    device float* output [[buffer(0)]],
    device const float* parameter [[buffer(1)]],
    constant uint* shape [[buffer(2)]],
    uint2 position [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], offset = shape[2], mode = shape[3];
    uint row = position.y, column = position.x;
    if (row >= count || column >= width) {
        return;
    }
    float value = parameter[offset + column];
    uint index = row * width + column;
    output[index] = mode == 0 ? value : output[index] + value;
}

kernel void activate(
    device const float* input [[buffer(0)]],
    device float* output [[buffer(1)]],
    constant uint* shape [[buffer(2)]],
    uint index [[thread_position_in_grid]])
{
    if (index >= shape[0]) {
        return;
    }
    output[index] = gelu(input[index]);
}

kernel void attend(
    device const float* mixed [[buffer(0)]],
    device float* context [[buffer(1)]],
    device const uint* tile [[buffer(2)]],
    device float* statistic [[buffer(3)]],
    constant uint* shape [[buffer(4)]],
    uint2 group [[threadgroup_position_in_grid]],
    uint2 place [[thread_position_in_threadgroup]])
{
    uint lane = place.x;
    uint width = shape[0], head = shape[1], store = shape[2];
    uint index = group.y;
    uint start = tile[3 * group.x], length = tile[3 * group.x + 1], first = tile[3 * group.x + 2];
    uint token = start + first + lane;
    bool active = first + lane < length;
    uint stride = 3 * width, offset = index * DIMENSION;
    float scale = rsqrt(float(DIMENSION));
    threadgroup float key[TILE][DIMENSION];
    threadgroup float value[TILE][DIMENSION];
    float query[DIMENSION];
    float accumulator[DIMENSION];
    for (uint column = 0; column < DIMENSION; column++) {
        query[column] = active ? mixed[token * stride + offset + column] * scale : 0.0f;
        accumulator[column] = 0.0f;
    }
    float maximum = -INFINITY;
    float total = 0.0f;
    for (uint base = 0; base < length; base += TILE) {
        uint count = min(uint(TILE), length - base);
        for (uint entry = lane; entry < count * DIMENSION; entry += TILE) {
            uint row = entry / DIMENSION, column = entry % DIMENSION;
            device const float* source = mixed + (start + base + row) * stride + offset + column;
            key[row][column] = source[width];
            value[row][column] = source[2 * width];
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
        float score[TILE];
        float local = -INFINITY;
        for (uint row = 0; row < TILE; row++) {
            float product = 0.0f;
            if (row < count) {
                for (uint column = 0; column < DIMENSION; column++) {
                    product += query[column] * key[row][column];
                }
            }
            score[row] = product;
            local = row < count ? max(local, product) : local;
        }
        float next = max(maximum, local);
        float correction = exp(maximum - next);
        total *= correction;
        for (uint column = 0; column < DIMENSION; column++) {
            accumulator[column] *= correction;
        }
        for (uint row = 0; row < TILE; row++) {
            if (row < count) {
                float weight = exp(score[row] - next);
                total += weight;
                for (uint column = 0; column < DIMENSION; column++) {
                    accumulator[column] += weight * value[row][column];
                }
            }
        }
        maximum = next;
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }
    if (!active) {
        return;
    }
    for (uint column = 0; column < DIMENSION; column++) {
        context[token * width + offset + column] = accumulator[column] / total;
    }
    if (store != 0) {
        statistic[token * head + index] = maximum + log(total);
    }
}

kernel void gather(
    device const float* input [[buffer(0)]],
    device const uint* row [[buffer(1)]],
    device float* output [[buffer(2)]],
    constant uint* shape [[buffer(3)]],
    uint2 position [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1];
    uint sample = position.y, column = position.x;
    if (sample >= count || column >= width) {
        return;
    }
    output[sample * width + column] = input[row[sample] * width + column];
}

kernel void point(
    device const uint* pointer [[buffer(0)]],
    device const float* unary [[buffer(1)]],
    device const float* query [[buffer(2)]],
    device const float* key [[buffer(3)]],
    device float* logit [[buffer(4)]],
    constant uint* shape [[buffer(5)]],
    uint index [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], dimension = shape[2], stride = shape[3];
    if (index >= count) {
        return;
    }
    uint kind = pointer[4 * index], head = pointer[4 * index + 1];
    uint left = pointer[4 * index + 2], right = pointer[4 * index + 3];
    if (kind == 0) {
        logit[index] = unary[left * width + head];
        return;
    }
    float total = 0.0f;
    for (uint column = 0; column < dimension; column++) {
        total += query[left * stride + head * dimension + column] * key[right * stride + head * dimension + column];
    }
    logit[index] = total / sqrt(float(dimension));
}

kernel void clear(
    device float* output [[buffer(0)]],
    constant uint* shape [[buffer(1)]],
    uint index [[thread_position_in_grid]])
{
    if (index >= shape[0]) {
        return;
    }
    output[index] = 0.0f;
}

kernel void derive(
    device float* delta [[buffer(0)]],
    device const float* activation [[buffer(1)]],
    constant uint* shape [[buffer(2)]],
    uint index [[thread_position_in_grid]])
{
    if (index >= shape[0]) {
        return;
    }
    delta[index] *= incline(activation[index]);
}

kernel void column(
    device const float* input [[buffer(0)]],
    device atomic_float* output [[buffer(1)]],
    constant uint* shape [[buffer(2)]],
    uint2 position [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], offset = shape[2], block = shape[3];
    uint target = position.x, start = position.y * block;
    if (target >= width || start >= count) {
        return;
    }
    uint end = min(start + block, count);
    float total = 0.0f;
    for (uint row = start; row < end; row++) {
        total += input[row * width + target];
    }
    atomic_fetch_add_explicit(output + offset + target, total, memory_order_relaxed);
}

kernel void unnorm(
    device const float* delta [[buffer(0)]],
    device const float* normal [[buffer(1)]],
    device const float* inverse [[buffer(2)]],
    device const float* parameter [[buffer(3)]],
    device float* output [[buffer(4)]],
    constant uint* shape [[buffer(5)]],
    uint row [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], scale = shape[2], mode = shape[3];
    if (row >= count) {
        return;
    }
    float mean = 0.0f, projection = 0.0f;
    for (uint column = 0; column < width; column++) {
        float scaled = delta[row * width + column] * parameter[scale + column];
        mean += scaled;
        projection += scaled * normal[row * width + column];
    }
    mean /= float(width);
    projection /= float(width);
    float factor = inverse[row];
    for (uint column = 0; column < width; column++) {
        uint index = row * width + column;
        float scaled = delta[index] * parameter[scale + column];
        float value = factor * (scaled - mean - normal[index] * projection);
        output[index] = mode == 0 ? value : output[index] + value;
    }
}

kernel void renorm(
    device const float* delta [[buffer(0)]],
    device const float* normal [[buffer(1)]],
    device atomic_float* gradient [[buffer(2)]],
    constant uint* shape [[buffer(3)]],
    uint2 position [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], scale = shape[2], shift = shape[3], block = shape[4];
    uint target = position.x, start = position.y * block;
    if (target >= width || start >= count) {
        return;
    }
    uint end = min(start + block, count);
    float stretch = 0.0f, offset = 0.0f;
    for (uint row = start; row < end; row++) {
        float value = delta[row * width + target];
        stretch += value * normal[row * width + target];
        offset += value;
    }
    atomic_fetch_add_explicit(gradient + scale + target, stretch, memory_order_relaxed);
    atomic_fetch_add_explicit(gradient + shift + target, offset, memory_order_relaxed);
}

kernel void unattend(
    device const float* mixed [[buffer(0)]],
    device const float* context [[buffer(1)]],
    device const float* delta [[buffer(2)]],
    device const uint* tile [[buffer(3)]],
    device const float* statistic [[buffer(4)]],
    device float* inner [[buffer(5)]],
    device float* output [[buffer(6)]],
    constant uint* shape [[buffer(7)]],
    uint2 group [[threadgroup_position_in_grid]],
    uint2 place [[thread_position_in_threadgroup]])
{
    uint lane = place.x;
    uint width = shape[0], head = shape[1];
    uint index = group.y;
    uint start = tile[3 * group.x], length = tile[3 * group.x + 1], first = tile[3 * group.x + 2];
    uint token = start + first + lane;
    bool active = first + lane < length;
    uint stride = 3 * width, offset = index * DIMENSION;
    float scale = rsqrt(float(DIMENSION));
    threadgroup float key[TILE][DIMENSION];
    threadgroup float value[TILE][DIMENSION];
    float query[DIMENSION];
    float change[DIMENSION];
    float gradient[DIMENSION];
    float projection = 0.0f;
    for (uint column = 0; column < DIMENSION; column++) {
        query[column] = active ? mixed[token * stride + offset + column] * scale : 0.0f;
        change[column] = active ? delta[token * width + offset + column] : 0.0f;
        projection += active ? change[column] * context[token * width + offset + column] : 0.0f;
        gradient[column] = 0.0f;
    }
    float logarithm = active ? statistic[token * head + index] : 0.0f;
    for (uint base = 0; base < length; base += TILE) {
        uint count = min(uint(TILE), length - base);
        for (uint entry = lane; entry < count * DIMENSION; entry += TILE) {
            uint row = entry / DIMENSION, column = entry % DIMENSION;
            device const float* source = mixed + (start + base + row) * stride + offset + column;
            key[row][column] = source[width];
            value[row][column] = source[2 * width];
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
        for (uint row = 0; row < TILE; row++) {
            if (row < count) {
                float score = 0.0f, product = 0.0f;
                for (uint column = 0; column < DIMENSION; column++) {
                    score += query[column] * key[row][column];
                    product += change[column] * value[row][column];
                }
                float weight = exp(score - logarithm) * (product - projection) * scale;
                for (uint column = 0; column < DIMENSION; column++) {
                    gradient[column] += weight * key[row][column];
                }
            }
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }
    if (!active) {
        return;
    }
    inner[token * head + index] = projection;
    for (uint column = 0; column < DIMENSION; column++) {
        output[token * stride + offset + column] = gradient[column];
    }
}

kernel void release(
    device const float* mixed [[buffer(0)]],
    device const float* delta [[buffer(1)]],
    device const uint* tile [[buffer(2)]],
    device const float* statistic [[buffer(3)]],
    device const float* inner [[buffer(4)]],
    device float* output [[buffer(5)]],
    constant uint* shape [[buffer(6)]],
    uint2 group [[threadgroup_position_in_grid]],
    uint2 place [[thread_position_in_threadgroup]])
{
    uint lane = place.x;
    uint width = shape[0], head = shape[1];
    uint index = group.y;
    uint start = tile[3 * group.x], length = tile[3 * group.x + 1], first = tile[3 * group.x + 2];
    uint token = start + first + lane;
    bool active = first + lane < length;
    uint stride = 3 * width, offset = index * DIMENSION;
    float scale = rsqrt(float(DIMENSION));
    threadgroup float query[TILE][DIMENSION];
    threadgroup float change[TILE][DIMENSION];
    threadgroup float logarithm[TILE];
    threadgroup float projection[TILE];
    float key[DIMENSION];
    float value[DIMENSION];
    float gradient[2][DIMENSION];
    for (uint column = 0; column < DIMENSION; column++) {
        key[column] = active ? mixed[token * stride + width + offset + column] : 0.0f;
        value[column] = active ? mixed[token * stride + 2 * width + offset + column] : 0.0f;
        gradient[0][column] = 0.0f;
        gradient[1][column] = 0.0f;
    }
    for (uint base = 0; base < length; base += TILE) {
        uint count = min(uint(TILE), length - base);
        for (uint entry = lane; entry < count * DIMENSION; entry += TILE) {
            uint row = entry / DIMENSION, column = entry % DIMENSION;
            uint source = start + base + row;
            query[row][column] = mixed[source * stride + offset + column] * scale;
            change[row][column] = delta[source * width + offset + column];
        }
        if (lane < count) {
            logarithm[lane] = statistic[(start + base + lane) * head + index];
            projection[lane] = inner[(start + base + lane) * head + index];
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
        for (uint row = 0; row < TILE; row++) {
            if (row < count) {
                float score = 0.0f, product = 0.0f;
                for (uint column = 0; column < DIMENSION; column++) {
                    score += query[row][column] * key[column];
                    product += change[row][column] * value[column];
                }
                float weight = exp(score - logarithm[row]);
                float slope = weight * (product - projection[row]);
                for (uint column = 0; column < DIMENSION; column++) {
                    gradient[0][column] += slope * query[row][column];
                    gradient[1][column] += weight * change[row][column];
                }
            }
        }
        threadgroup_barrier(mem_flags::mem_threadgroup);
    }
    if (!active) {
        return;
    }
    for (uint column = 0; column < DIMENSION; column++) {
        output[token * stride + width + offset + column] = gradient[0][column];
        output[token * stride + 2 * width + offset + column] = gradient[1][column];
    }
}

kernel void spread(
    device const float* input [[buffer(0)]],
    device const uint* row [[buffer(1)]],
    device float* output [[buffer(2)]],
    constant uint* shape [[buffer(3)]],
    uint2 position [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1];
    uint sample = position.y, column = position.x;
    if (sample >= count || column >= width) {
        return;
    }
    output[row[sample] * width + column] += input[sample * width + column];
}

kernel void unpoint(
    device const uint* pointer [[buffer(0)]],
    device const float* delta [[buffer(1)]],
    device const float* query [[buffer(2)]],
    device const float* key [[buffer(3)]],
    device atomic_float* choice [[buffer(4)]],
    device atomic_float* request [[buffer(5)]],
    device atomic_float* answer [[buffer(6)]],
    constant uint* shape [[buffer(7)]],
    uint index [[thread_position_in_grid]])
{
    uint count = shape[0], width = shape[1], dimension = shape[2], stride = shape[3];
    if (index >= count) {
        return;
    }
    uint kind = pointer[4 * index], head = pointer[4 * index + 1];
    uint left = pointer[4 * index + 2], right = pointer[4 * index + 3];
    float change = delta[index];
    if (kind == 0) {
        atomic_fetch_add_explicit(choice + left * width + head, change, memory_order_relaxed);
        return;
    }
    float scale = change / sqrt(float(dimension));
    for (uint column = 0; column < dimension; column++) {
        uint first = left * stride + head * dimension + column;
        uint second = right * stride + head * dimension + column;
        atomic_fetch_add_explicit(request + first, scale * key[second], memory_order_relaxed);
        atomic_fetch_add_explicit(answer + second, scale * query[first], memory_order_relaxed);
    }
}

kernel void unembed(
    device const uint* order [[buffer(0)]],
    device const uint* offset [[buffer(1)]],
    device const uint* target [[buffer(2)]],
    device const float* delta [[buffer(3)]],
    device float* gradient [[buffer(4)]],
    constant uint* shape [[buffer(5)]],
    uint2 position [[thread_position_in_grid]])
{
    uint width = shape[0], count = shape[1];
    uint column = position.x, value = position.y;
    if (column >= width || value >= count) {
        return;
    }
    float total = 0.0f;
    for (uint entry = offset[value]; entry < offset[value + 1]; entry++) {
        total += delta[order[entry] * width + column];
    }
    gradient[target[value] + column] += total;
}

kernel void square(
    device const float* gradient [[buffer(0)]],
    device atomic_float* total [[buffer(1)]],
    constant uint* shape [[buffer(2)]],
    uint index [[thread_position_in_grid]],
    uint lane [[thread_index_in_simdgroup]])
{
    float value = index < shape[0] ? gradient[index] : 0.0f;
    float sum = simd_sum(value * value);
    if (lane == 0) {
        atomic_fetch_add_explicit(total, sum, memory_order_relaxed);
    }
}

kernel void adam(
    device float* parameter [[buffer(0)]],
    device const float* gradient [[buffer(1)]],
    device float* moment [[buffer(2)]],
    device float* velocity [[buffer(3)]],
    device const uint* decay [[buffer(4)]],
    device const float* total [[buffer(5)]],
    device const float* setting [[buffer(6)]],
    constant uint* shape [[buffer(7)]],
    uint index [[thread_position_in_grid]])
{
    if (index >= shape[0]) {
        return;
    }
    float rate = setting[0], weight = setting[1], first = setting[2], second = setting[3];
    float epsilon = setting[4], clip = setting[5], early = setting[6], late = setting[7];
    float norm = sqrt(total[0]);
    float factor = norm > clip && norm > 0.0f ? clip / norm : 1.0f;
    float change = gradient[index] * factor;
    float value = parameter[index];
    if (decay[index] != 0) {
        value -= rate * weight * value;
    }
    float momentum = first * moment[index] + (1.0f - first) * change;
    float variance = second * velocity[index] + (1.0f - second) * change * change;
    moment[index] = momentum;
    velocity[index] = variance;
    parameter[index] = value - rate * (momentum / early) / (sqrt(variance / late) + epsilon);
}
