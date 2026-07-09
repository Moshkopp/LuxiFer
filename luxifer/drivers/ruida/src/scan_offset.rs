//! Geschwindigkeitsabhängige Reversal-Korrektur (Scanning Offset) — Ruida-lokal.
//!
//! Problem (an Hardware beobachtet): Beim bidirektionalen Rastern feuert die
//! Röhre mit einer kleinen, konstanten Zeitverzögerung, die als Weg umschlägt
//! (Versatz = Geschwindigkeit × Verzögerung). Hin- und Rückzeile werden dadurch
//! entgegengesetzt verschoben — die Kanten fransen aus. Der Effekt wächst
//! linear mit der Geschwindigkeit.
//!
//! Korrektur: jede Zeile um einen geschwindigkeitsabhängigen Offset IN
//! Fahrtrichtung verschieben. Modelliert als Tabelle von Stützpunkten
//! (`speed_mm_s → offset_mm`), linear interpoliert und über die Ränder
//! extrapoliert.
//!
//! **Bewusst hier, nicht im Core:** Der Offset ist ein physikalischer Kennwert
//! der konkreten Maschine (Antrieb/Optik), kein Job-Inhalt. Der `JobPlan` bleibt
//! Ideal-Soll-Geometrie; die Korrektur macht allein dieser Treiber (ADR 0006 §6,
//! ADR 0007). Die Kalibrierung kommt später aus dem Laser-Profil.

/// Ein Stützpunkt der Offset-Kurve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanOffsetPoint {
    pub speed_mm_s: f64,
    pub offset_mm: f64,
}

/// Geschwindigkeitsabhängige Offset-Korrektur (Tabelle + Interpolation).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScanOffset {
    /// Aktiv? `false` → immer 0 (eingetragene Punkte werden ignoriert).
    pub enabled: bool,
    /// Stützpunkte, beliebig viele. Leer → keine Korrektur.
    pub points: Vec<ScanOffsetPoint>,
}

impl ScanOffset {
    /// Linearer Faktor: `offset_mm = speed_mm_s × mm_per_mm_s`. Als Tabelle mit
    /// zwei Punkten (0 und `ref_speed`), damit später Messpunkte ergänzbar sind.
    pub fn from_linear(mm_per_mm_s: f64, ref_speed: f64) -> Self {
        Self {
            enabled: true,
            points: vec![
                ScanOffsetPoint {
                    speed_mm_s: 0.0,
                    offset_mm: 0.0,
                },
                ScanOffsetPoint {
                    speed_mm_s: ref_speed,
                    offset_mm: ref_speed * mm_per_mm_s,
                },
            ],
        }
    }

    /// Interpolierter Offset (mm) für eine Geschwindigkeit. Außerhalb der
    /// Stützpunkte wird über das jeweilige Randsegment extrapoliert; bei genau
    /// einem Punkt gilt dessen Offset konstant. Deaktiviert/leer → 0.
    pub fn offset_mm(&self, speed_mm_s: f64) -> f64 {
        if !self.enabled || self.points.is_empty() {
            return 0.0;
        }
        let mut pts = self.points.clone();
        pts.sort_by(|a, b| a.speed_mm_s.total_cmp(&b.speed_mm_s));
        if pts.len() == 1 {
            return pts[0].offset_mm;
        }
        // Passendes Segment finden (oder das nächste Randsegment für Extrapolation).
        for i in 0..pts.len() - 1 {
            let (p0, p1) = (pts[i], pts[i + 1]);
            if speed_mm_s <= p1.speed_mm_s || i == pts.len() - 2 {
                if p1.speed_mm_s == p0.speed_mm_s {
                    return p0.offset_mm;
                }
                let t = (speed_mm_s - p0.speed_mm_s) / (p1.speed_mm_s - p0.speed_mm_s);
                return p0.offset_mm + t * (p1.offset_mm - p0.offset_mm);
            }
        }
        pts[pts.len() - 1].offset_mm
    }

    /// Interpolierter Offset in µm (gerundet), für die Geometrie-Erzeugung.
    pub fn offset_um(&self, speed_mm_s: f64) -> i32 {
        (self.offset_mm(speed_mm_s) * 1000.0).round() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_interpoliert_und_extrapoliert() {
        let so = ScanOffset::from_linear(0.001, 100.0); // 0.1mm @ 100 mm/s
        assert!((so.offset_mm(100.0) - 0.1).abs() < 1e-9);
        assert!((so.offset_mm(20.0) - 0.02).abs() < 1e-9);
        assert_eq!(so.offset_um(100.0), 100);
        // Über den letzten Stützpunkt hinaus wird extrapoliert.
        assert!((so.offset_mm(300.0) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn deaktiviert_liefert_null() {
        let so = ScanOffset {
            enabled: false,
            points: vec![ScanOffsetPoint {
                speed_mm_s: 100.0,
                offset_mm: 0.2,
            }],
        };
        assert_eq!(so.offset_mm(100.0), 0.0);
    }

    #[test]
    fn echte_tabelle_nichtlinear() {
        let so = ScanOffset {
            enabled: true,
            points: vec![
                ScanOffsetPoint {
                    speed_mm_s: 20.0,
                    offset_mm: 0.0,
                },
                ScanOffsetPoint {
                    speed_mm_s: 100.0,
                    offset_mm: 0.15,
                },
                ScanOffsetPoint {
                    speed_mm_s: 300.0,
                    offset_mm: 0.6,
                },
            ],
        };
        assert_eq!(so.offset_mm(20.0), 0.0);
        assert!((so.offset_mm(60.0) - 0.075).abs() < 1e-9); // zwischen 20 und 100
    }

    #[test]
    fn leer_liefert_null() {
        assert_eq!(ScanOffset::default().offset_mm(100.0), 0.0);
    }
}
