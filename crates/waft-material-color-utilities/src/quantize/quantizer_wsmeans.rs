// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Ported from `lib/quantize/quantizer_wsmeans.dart` of material_color_utilities 0.13.0.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

use indexmap::IndexMap;

use crate::dart_math::{self, Random};
use crate::quantize::quantizer::QuantizerResult;
use crate::quantize::src::point_provider::PointProvider;

#[derive(Clone, Copy, Debug)]
pub struct DistanceAndIndex {
    pub distance: f64,
    pub index: i64,
}

impl DistanceAndIndex {
    pub fn new(distance: f64, index: i64) -> DistanceAndIndex {
        DistanceAndIndex { distance, index }
    }

    /// Dart's `compareTo`, the order `sort` puts a row of the distance matrix in.
    pub fn compare_to(&self, other: &DistanceAndIndex) -> Ordering {
        if self.distance < other.distance {
            Ordering::Less
        } else if self.distance > other.distance {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

pub struct QuantizerWsmeans;

impl QuantizerWsmeans {
    pub const DEBUG: bool = false;

    pub fn debug_log(log: &str) {
        if Self::DEBUG {
            println!("{log}");
        }
    }

    /// Dart's defaults: no `starting_clusters`, `PointProviderLab`, 5 `max_iterations`, and
    /// `false` for `return_input_pixel_to_cluster_pixel`.
    // The index loops stay Dart's, as the package keeps them alike across its languages.
    #[allow(clippy::needless_range_loop)]
    pub fn quantize(
        input_pixels: &[u32],
        max_colors: i64,
        starting_clusters: &[u32],
        point_provider: &dyn PointProvider,
        max_iterations: i64,
        return_input_pixel_to_cluster_pixel: bool,
    ) -> QuantizerResult {
        let mut pixel_to_count: HashMap<u32, i64> = HashMap::new();
        let mut points: Vec<[f64; 3]> = Vec::new();
        let mut pixels: Vec<u32> = Vec::new();
        let mut point_count: usize = 0;
        for input_pixel in input_pixels {
            let pixel_count = {
                let count = pixel_to_count.entry(*input_pixel).or_insert(0);
                *count += 1;
                *count
            };
            if pixel_count == 1 {
                point_count += 1;
                points.push(point_provider.from_int(*input_pixel));
                pixels.push(*input_pixel);
            }
        }

        let mut counts: Vec<i64> = vec![0; point_count];
        for i in 0..point_count {
            let pixel = pixels[i];
            let count = pixel_to_count[&pixel];
            counts[i] = count;
        }

        let cluster_count = max_colors.min(point_count as i64) as usize;

        let mut clusters: Vec<[f64; 3]> = starting_clusters
            .iter()
            .map(|e| point_provider.from_int(*e))
            .collect();
        let additional_clusters_needed = cluster_count as i64 - clusters.len() as i64;
        if additional_clusters_needed > 0 {
            let mut random = Random::new(0x42688);
            let mut indices: Vec<usize> = Vec::new();
            for _ in 0..additional_clusters_needed {
                // Use existing points rather than generating random centroids.
                //
                // KMeans is extremely sensitive to initial clusters. This quantizer
                // is meant to be used with a Wu quantizer that provides initial
                // centroids, but Wu is very slow on unscaled images and when extracting
                // more than 256 colors.
                //
                // Here, we can safely assume that more than 256 colors were requested
                // for extraction. Generating random centroids tends to lead to many
                // "empty" centroids, as the random centroids are nowhere near any pixels
                // in the image, and the centroids from Wu are very refined and close
                // to pixels in the image.
                //
                // Rather than generate random centroids, we'll pick centroids that
                // are actual pixels in the image, and avoid duplicating centroids.

                let mut index = random.next_int(points.len() as u64) as usize;
                while indices.contains(&index) {
                    index = random.next_int(points.len() as u64) as usize;
                }
                indices.push(index);
            }

            for index in indices {
                clusters.push(points[index]);
            }
        }
        Self::debug_log(&format!(
            "have {} starting clusters, {} points",
            clusters.len(),
            points.len()
        ));
        let mut cluster_indices: Vec<usize> = (0..point_count)
            .map(|index| index % cluster_count)
            .collect();
        let mut index_matrix: Vec<Vec<i64>> = vec![vec![0; cluster_count]; cluster_count];

        let mut distance_to_index_matrix: Vec<Vec<DistanceAndIndex>> = (0..cluster_count)
            .map(|_| {
                (0..cluster_count)
                    .map(|index| DistanceAndIndex::new(0.0, index as i64))
                    .collect()
            })
            .collect();

        let mut pixel_count_sums: Vec<i64> = vec![0; cluster_count];
        for iteration in 0..max_iterations {
            if Self::DEBUG {
                for i in 0..cluster_count {
                    pixel_count_sums[i] = 0;
                }
                for i in 0..point_count {
                    let cluster_index = cluster_indices[i];
                    let count = counts[i];
                    pixel_count_sums[cluster_index] += count;
                }
                let mut empty_clusters = 0;
                for cluster in 0..cluster_count {
                    if pixel_count_sums[cluster] == 0 {
                        empty_clusters += 1;
                    }
                }
                Self::debug_log(&format!(
                    "starting iteration {}; {empty_clusters} clusters are empty of {cluster_count}",
                    iteration + 1
                ));
            }

            let mut points_moved = 0;
            for i in 0..cluster_count {
                for j in (i + 1)..cluster_count {
                    let distance = point_provider.distance(&clusters[i], &clusters[j]);
                    distance_to_index_matrix[j][i].distance = distance;
                    distance_to_index_matrix[j][i].index = i as i64;
                    distance_to_index_matrix[i][j].distance = distance;
                    distance_to_index_matrix[i][j].index = j as i64;
                }
                dart_math::sort(
                    &mut distance_to_index_matrix[i],
                    DistanceAndIndex::compare_to,
                );
                for j in 0..cluster_count {
                    index_matrix[i][j] = distance_to_index_matrix[i][j].index;
                }
            }

            for i in 0..point_count {
                let point = points[i];
                let previous_cluster_index = cluster_indices[i];
                let previous_cluster = clusters[previous_cluster_index];
                let previous_distance = point_provider.distance(&point, &previous_cluster);
                let mut minimum_distance = previous_distance;
                let mut new_cluster_index: i64 = -1;
                for j in 0..cluster_count {
                    if distance_to_index_matrix[previous_cluster_index][j].distance
                        >= 4.0 * previous_distance
                    {
                        continue;
                    }
                    let distance = point_provider.distance(&point, &clusters[j]);
                    if distance < minimum_distance {
                        minimum_distance = distance;
                        new_cluster_index = j as i64;
                    }
                }
                if new_cluster_index != -1 {
                    points_moved += 1;
                    cluster_indices[i] = new_cluster_index as usize;
                }
            }

            if points_moved == 0 && iteration > 0 {
                Self::debug_log(&format!("terminated after {iteration} k-means iterations"));
                break;
            }

            Self::debug_log(&format!("iteration {} moved {points_moved}", iteration + 1));
            let mut component_a_sums: Vec<f64> = vec![0.0; cluster_count];
            let mut component_b_sums: Vec<f64> = vec![0.0; cluster_count];
            let mut component_c_sums: Vec<f64> = vec![0.0; cluster_count];

            for i in 0..cluster_count {
                pixel_count_sums[i] = 0;
            }
            for i in 0..point_count {
                let cluster_index = cluster_indices[i];
                let point = points[i];
                let count = counts[i];
                pixel_count_sums[cluster_index] += count;
                component_a_sums[cluster_index] += point[0] * count as f64;
                component_b_sums[cluster_index] += point[1] * count as f64;
                component_c_sums[cluster_index] += point[2] * count as f64;
            }
            for i in 0..cluster_count {
                let count = pixel_count_sums[i];
                if count == 0 {
                    clusters[i] = [0.0, 0.0, 0.0];
                    continue;
                }
                let a = component_a_sums[i] / count as f64;
                let b = component_b_sums[i] / count as f64;
                let c = component_c_sums[i] / count as f64;
                clusters[i] = [a, b, c];
            }
        }

        let mut cluster_argbs: Vec<u32> = Vec::new();
        let mut cluster_populations: Vec<i64> = Vec::new();
        for i in 0..cluster_count {
            let count = pixel_count_sums[i];
            if count == 0 {
                continue;
            }

            let possible_new_cluster = point_provider.to_int(&clusters[i]);
            if cluster_argbs.contains(&possible_new_cluster) {
                continue;
            }

            cluster_argbs.push(possible_new_cluster);
            cluster_populations.push(count);
        }
        Self::debug_log(&format!(
            "kmeans finished and generated {} clusters; {cluster_count} were requested",
            cluster_argbs.len()
        ));

        let mut input_pixel_to_cluster_pixel: IndexMap<u32, u32> = IndexMap::new();
        if return_input_pixel_to_cluster_pixel {
            // Dart's stopwatch is read only by the debug log; std's clock panics on the web.
            let stopwatch = Self::DEBUG.then(Instant::now);
            for i in 0..pixels.len() {
                let input_pixel = pixels[i];
                let cluster_index = cluster_indices[i];
                let cluster = clusters[cluster_index];
                let cluster_pixel = point_provider.to_int(&cluster);
                input_pixel_to_cluster_pixel.insert(input_pixel, cluster_pixel);
            }
            if let Some(stopwatch) = stopwatch {
                Self::debug_log(&format!(
                    "took {} ms to create input to cluster map",
                    stopwatch.elapsed().as_millis()
                ));
            }
        }

        QuantizerResult::new(cluster_argbs.into_iter().zip(cluster_populations).collect())
            .input_pixel_to_cluster_pixel(input_pixel_to_cluster_pixel)
    }
}

#[cfg(test)]
mod tests {
    // Ported from `test/quantizer_wsmeans_test.dart`.
    use super::*;
    use crate::quantize::src::point_provider_lab::PointProviderLab;

    const RED: u32 = 0xffff0000;
    const GREEN: u32 = 0xff00ff00;
    const BLUE: u32 = 0xff0000ff;
    const MAX_COLORS: i64 = 256;

    fn colors_of(pixels: &[u32]) -> Vec<u32> {
        let result =
            QuantizerWsmeans::quantize(pixels, MAX_COLORS, &[], &PointProviderLab, 5, false);
        result.color_to_count.keys().copied().collect()
    }

    #[test]
    fn one_rando() {
        let colors = colors_of(&[0xff141216]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], 0xff141216);
    }

    #[test]
    fn one_red() {
        let colors = colors_of(&[RED]);
        assert_eq!(colors.len(), 1);
    }

    #[test]
    fn one_red_is_red() {
        let colors = colors_of(&[RED]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], RED);
    }

    #[test]
    fn one_green() {
        let colors = colors_of(&[GREEN]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], GREEN);
    }

    #[test]
    fn one_blue() {
        let colors = colors_of(&[BLUE]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], BLUE);
    }

    #[test]
    fn five_blue() {
        let colors = colors_of(&[BLUE, BLUE, BLUE, BLUE, BLUE]);
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0], BLUE);
    }

    // Not in the package's tests: with no starting clusters, the seeded `Random` picks the
    // starting clusters. The expected values are the Dart package's, printed by
    // `QuantizerWsmeans.quantize(pixels, 4, returnInputPixelToClusterPixel: true)` on Dart 3.13.1.
    #[test]
    fn random_starting_clusters_give_dart_s_clusters() {
        let pixels = [
            0xff102030, 0xff405060, 0xff708090, 0xffa0b0c0, 0xffd0e0f0, 0xff332211, 0xff665544,
            0xff998877, 0xffccbbaa, 0xffff0000, 0xff00ff00, 0xff0000ff, 0xff102030, 0xffff0000,
        ];
        let result = QuantizerWsmeans::quantize(&pixels, 4, &[], &PointProviderLab, 5, true);
        let color_to_count: Vec<(u32, i64)> = result
            .color_to_count
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(
            color_to_count,
            [
                (0xff4b494c, 7),
                (0xffff0000, 2),
                (0xffad9ad8, 4),
                (0xff00ff00, 1)
            ]
        );
        let input_pixel_to_cluster_pixel: Vec<(u32, u32)> = result
            .input_pixel_to_cluster_pixel
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        assert_eq!(
            input_pixel_to_cluster_pixel,
            [
                (0xff102030, 0xff4b494c),
                (0xff405060, 0xff4b494c),
                (0xff708090, 0xff4b494c),
                (0xffa0b0c0, 0xffad9ad8),
                (0xffd0e0f0, 0xffad9ad8),
                (0xff332211, 0xff4b494c),
                (0xff665544, 0xff4b494c),
                (0xff998877, 0xff4b494c),
                (0xffccbbaa, 0xffad9ad8),
                (0xffff0000, 0xffff0000),
                (0xff00ff00, 0xff00ff00),
                (0xff0000ff, 0xffad9ad8),
            ]
        );
    }
}
