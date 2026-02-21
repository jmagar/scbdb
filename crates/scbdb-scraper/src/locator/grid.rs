//! Geographic grid search infrastructure.
//!
//! Generates uniform lat/lng grid points for systematic store locator sweeps.
//! Longitude step adjusts for latitude curvature so physical spacing stays ~equal.

use std::f64::consts::PI;

const MILES_PER_LAT_DEGREE: f64 = 69.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPoint {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone)]
pub struct GridConfig {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
    /// Physical distance between adjacent grid points in miles.
    pub step_miles: f64,
}

impl GridConfig {
    /// SC + immediate neighbors. 30-mile step → ~60 points.
    pub fn sc_region() -> Self {
        Self {
            min_lat: 32.0,
            max_lat: 35.2,
            min_lng: -83.4,
            max_lng: -78.5,
            step_miles: 30.0,
        }
    }

    /// CONUS at 200-mile step → ~100–150 points. Pair with 100-mile search radius.
    pub fn conus_coarse() -> Self {
        Self {
            min_lat: 24.4,
            max_lat: 49.4,
            min_lng: -125.0,
            max_lng: -66.9,
            step_miles: 200.0,
        }
    }
}

/// Strategic US city centers covering all major population regions including the Southeast.
/// Shared by Destini and VTInfo as their default search origin set.
pub const STRATEGIC_US_POINTS: &[GridPoint] = &[
    GridPoint {
        lat: 44.977_8,
        lng: -93.265_0,
    }, // Minneapolis — Upper Midwest
    GridPoint {
        lat: 39.828_3,
        lng: -98.579_5,
    }, // Kansas — US geographic center
    GridPoint {
        lat: 34.052_2,
        lng: -118.243_7,
    }, // Los Angeles — West Coast
    GridPoint {
        lat: 40.712_8,
        lng: -74.006_0,
    }, // New York — Northeast
    GridPoint {
        lat: 41.878_1,
        lng: -87.629_8,
    }, // Chicago — Great Lakes
    GridPoint {
        lat: 29.760_4,
        lng: -95.369_8,
    }, // Houston — Gulf Coast
    GridPoint {
        lat: 39.739_2,
        lng: -104.990_3,
    }, // Denver — Mountain West
    GridPoint {
        lat: 33.448_4,
        lng: -112.074_0,
    }, // Phoenix — Southwest
    GridPoint {
        lat: 35.227_1,
        lng: -80.843_1,
    }, // Charlotte — Southeast (SC, NC, GA, VA)
];

/// Generate a uniform lat/lng grid across the given bounds.
///
/// Longitude step narrows per latitude band so each column covers the same
/// physical ~`step_miles` distance regardless of where on the map it falls.
pub fn generate_grid(config: &GridConfig) -> Vec<GridPoint> {
    let lat_step = config.step_miles / MILES_PER_LAT_DEGREE;
    let mut points = Vec::new();
    let mut lat = config.min_lat;
    while lat <= config.max_lat + lat_step * 0.5 {
        let lng_step = config.step_miles / (MILES_PER_LAT_DEGREE * (lat * PI / 180.0).cos());
        let mut lng = config.min_lng;
        while lng <= config.max_lng + lng_step * 0.5 {
            points.push(GridPoint { lat, lng });
            lng += lng_step;
        }
        lat += lat_step;
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_sc_region_covers_bounds() {
        let cfg = GridConfig::sc_region();
        let pts = generate_grid(&cfg);
        assert!(!pts.is_empty());
        for p in &pts {
            // Allow up to one step_miles overshoot on each edge
            assert!(p.lat >= cfg.min_lat - 0.5 && p.lat <= cfg.max_lat + 1.0);
            assert!(p.lng >= cfg.min_lng - 0.5 && p.lng <= cfg.max_lng + 2.0);
        }
    }

    #[test]
    fn grid_conus_coarse_has_reasonable_count() {
        let pts = generate_grid(&GridConfig::conus_coarse());
        // 200-mile step across CONUS: expect 50–300 points
        assert!(pts.len() >= 50 && pts.len() <= 400, "got {}", pts.len());
    }

    #[test]
    fn strategic_us_points_includes_southeast() {
        // Charlotte must be present for SC brand coverage
        let has_charlotte = STRATEGIC_US_POINTS
            .iter()
            .any(|p| (p.lat - 35.227_1).abs() < 0.01 && (p.lng - (-80.843_1)).abs() < 0.01);
        assert!(
            has_charlotte,
            "Charlotte (SE coverage) missing from STRATEGIC_US_POINTS"
        );
    }

    #[test]
    fn grid_step_adjusts_for_latitude() {
        // At higher latitude, same lng range needs fewer columns (wider degree-per-mile)
        let cfg_low = GridConfig {
            min_lat: 10.0,
            max_lat: 10.0,
            min_lng: 0.0,
            max_lng: 100.0,
            step_miles: 100.0,
        };
        let cfg_high = GridConfig {
            min_lat: 60.0,
            max_lat: 60.0,
            min_lng: 0.0,
            max_lng: 100.0,
            step_miles: 100.0,
        };
        let pts_low = generate_grid(&cfg_low);
        let pts_high = generate_grid(&cfg_high);
        assert!(pts_high.len() < pts_low.len());
    }
}
