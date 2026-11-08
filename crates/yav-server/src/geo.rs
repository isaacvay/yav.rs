//! # YAV Geospatial & Geofencing Engine
//!
//! Haversine great-circle distance, ray-casting point-in-polygon geofencing,
//! and k-NN / radius spatial indexing (rivaling Turf.js, PostGIS helpers, and
//! Uber H3 proximity lookups).

use serde::{Deserialize, Serialize};

const EARTH_RADIUS_KM: f64 = 6371.0088;

/// WGS84 geographic coordinate (`lat`, `lng` in degrees).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

impl GeoPoint {
    pub const fn new(lat: f64, lng: f64) -> Self {
        Self { lat, lng }
    }

    /// Great-circle Haversine distance in kilometers between two coordinates.
    pub fn distance_km(&self, other: &GeoPoint) -> f64 {
        let d_lat = (other.lat - self.lat).to_radians();
        let d_lng = (other.lng - self.lng).to_radians();
        let lat1 = self.lat.to_radians();
        let lat2 = other.lat.to_radians();

        let a = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).max(0.0).sqrt());
        EARTH_RADIUS_KM * c
    }

    /// Great-circle Haversine distance in meters.
    pub fn distance_meters(&self, other: &GeoPoint) -> f64 {
        self.distance_km(other) * 1000.0
    }
}

/// Closed polygon geofence defined by exterior ring vertices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoPolygon {
    pub vertices: Vec<GeoPoint>,
}

impl GeoPolygon {
    pub fn new(vertices: Vec<GeoPoint>) -> Self {
        Self { vertices }
    }

    /// Ray-casting point-in-polygon test (`true` if `pt` lies inside the geofence).
    pub fn contains(&self, pt: &GeoPoint) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let vi = &self.vertices[i];
            let vj = &self.vertices[j];
            let intersects = ((vi.lat > pt.lat) != (vj.lat > pt.lat))
                && (pt.lng
                    < (vj.lng - vi.lng) * (pt.lat - vi.lat) / (vj.lat - vi.lat + f64::EPSILON)
                        + vi.lng);
            if intersects {
                inside = !inside;
            }
            j = i;
        }
        inside
    }
}

/// Spatial entry returned from proximity queries.
#[derive(Debug, Clone)]
pub struct SpatialMatch<T> {
    pub id: String,
    pub point: GeoPoint,
    pub distance_km: f64,
    pub item: T,
}

/// In-memory spatial index supporting radius (`find_within_km`) and `nearest_k` queries.
#[derive(Debug, Clone)]
pub struct SpatialIndex<T> {
    entries: Vec<(String, GeoPoint, T)>,
}

impl<T: Clone> Default for SpatialIndex<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> SpatialIndex<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert an item at a geographic coordinate.
    pub fn insert(&mut self, id: impl Into<String>, point: GeoPoint, item: T) {
        self.entries.push((id.into(), point, item));
    }

    /// Find all items within `radius_km` of `center`, sorted ascending by distance.
    pub fn find_within_km(&self, center: &GeoPoint, radius_km: f64) -> Vec<SpatialMatch<T>> {
        let mut matches: Vec<SpatialMatch<T>> = self
            .entries
            .iter()
            .filter_map(|(id, pt, item)| {
                let d = center.distance_km(pt);
                if d <= radius_km {
                    Some(SpatialMatch {
                        id: id.clone(),
                        point: *pt,
                        distance_km: d,
                        item: item.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();
        matches.sort_by(|a, b| {
            a.distance_km
                .partial_cmp(&b.distance_km)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches
    }

    /// Find the `k` nearest neighbors to `center`.
    pub fn nearest_k(&self, center: &GeoPoint, k: usize) -> Vec<SpatialMatch<T>> {
        let mut all: Vec<SpatialMatch<T>> = self
            .entries
            .iter()
            .map(|(id, pt, item)| SpatialMatch {
                id: id.clone(),
                point: *pt,
                distance_km: center.distance_km(pt),
                item: item.clone(),
            })
            .collect();
        all.sort_by(|a, b| {
            a.distance_km
                .partial_cmp(&b.distance_km)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.truncate(k);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_geofence_and_spatial_index() {
        let paris = GeoPoint::new(48.8566, 2.3522);
        let london = GeoPoint::new(51.5074, -0.1278);
        let dist = paris.distance_km(&london);
        assert!(dist > 330.0 && dist < 350.0);

        let box_fence = GeoPolygon::new(vec![
            GeoPoint::new(48.80, 2.25),
            GeoPoint::new(48.80, 2.45),
            GeoPoint::new(48.90, 2.45),
            GeoPoint::new(48.90, 2.25),
        ]);
        assert!(box_fence.contains(&paris));
        assert!(!box_fence.contains(&london));

        let mut idx = SpatialIndex::new();
        idx.insert("paris", paris, "FR");
        idx.insert("london", london, "UK");
        let nearby = idx.find_within_km(&paris, 50.0);
        assert_eq!(nearby.len(), 1);
        assert_eq!(nearby[0].id, "paris");
    }
}
