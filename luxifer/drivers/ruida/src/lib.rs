//! Ruida-Treiber: übersetzt den geräteunabhängigen [`JobPlan`] in einen
//! vollständigen Ruida-Binärjob (RDC6445G).
//!
//! Kennt nur `luxifer-core` (ADR 0001). Job-Rahmen (Preamble, Layer-Config,
//! Settings-Block, Geometrie, Trailer) folgt der HW-verifizierten
//! ThorBurn-Referenz. Kodierung siehe [`protocol`].
//!
//! Start-Modus ist derzeit „Absolut" (kein Anker-Offset). Andere Startmodi und
//! der Fokus-Z-Move sind Ausbaustufen.

pub mod protocol;
pub mod transport;

pub use transport::{RuidaTransport, TransportError};

use luxifer_core::{JobLayer, JobPlan, Layer, LayerWork, MachineDriver};
use protocol::*;

/// Der Ruida-Treiber.
#[derive(Debug, Clone, Default)]
pub struct RuidaDriver;

impl RuidaDriver {
    /// Baut den vollständigen, ungeswizzelten Job (Preamble … Trailer).
    pub fn build_job(&self, plan: &JobPlan) -> Vec<u8> {
        let mut j = Vec::new();

        // Gesamt-Bounding-Box in µm.
        let (minx, miny, maxx, maxy) = plan.bbox.unwrap_or((0.0, 0.0, 0.0, 0.0));
        let (minx_um, miny_um) = (mm_to_um(minx), mm_to_um(miny));
        let (maxx_um, maxy_um) = (mm_to_um(maxx), mm_to_um(maxy));
        let max_idx = plan.layers.len().saturating_sub(1) as u8;

        // 1. Preamble
        j.extend(compile_preamble(minx_um, miny_um, maxx_um, maxy_um));
        // 2. Layer-Config
        j.extend(compile_layer_config(&plan.layers, max_idx));
        // 3. Geometrie (pro Layer Settings-Block + Bahnen)
        j.extend(self.compile_geometry(&plan.layers));
        // 4. Trailer + Dateisumme
        j.extend_from_slice(&[0xEB, 0xE7, 0x00]);
        j.extend_from_slice(&[0xDA, 0x01, 0x06, 0x20]);
        j.extend(encode_coord(minx_um));
        j.extend(encode_coord(miny_um));
        let sum = recompute_file_sum(&j);
        j.extend(sum);

        j
    }

    fn compile_geometry(&self, layers: &[JobLayer]) -> Vec<u8> {
        let mut j = Vec::new();
        for (k, jl) in layers.iter().enumerate() {
            let idx = k as u8;
            let is_cut = matches!(jl.work, LayerWork::Cut { .. });
            if k > 0 {
                j.extend_from_slice(&[0xE7, 0x00]);
            }
            write_settings_block(
                &mut j,
                is_cut,
                idx,
                jl.speed_mm_s,
                jl.min_power_pct,
                jl.power_pct,
            );

            let passes = jl.passes.max(1);
            for _ in 0..passes {
                match &jl.work {
                    LayerWork::Cut { paths } => {
                        for path in paths {
                            if path.points.is_empty() {
                                continue;
                            }
                            let (x0, y0) = path.points[0];
                            j.extend(cmd_move_abs(mm_to_um(x0), mm_to_um(y0)));
                            for &(x, y) in &path.points[1..] {
                                j.extend(cmd_cut_abs(mm_to_um(x), mm_to_um(y)));
                            }
                            if path.closed {
                                j.extend(cmd_cut_abs(mm_to_um(x0), mm_to_um(y0)));
                            }
                        }
                    }
                    LayerWork::Fill { segments } => {
                        for seg in segments {
                            j.extend(cmd_move_abs(mm_to_um(seg.x0), mm_to_um(seg.y)));
                            j.extend(cmd_cut_abs(mm_to_um(seg.x1), mm_to_um(seg.y)));
                        }
                    }
                }
            }
        }
        j
    }
}

impl MachineDriver for RuidaDriver {
    fn name(&self) -> &str {
        "Ruida"
    }

    fn compile(&self, plan: &JobPlan, _layers: &[Layer]) -> Result<Vec<u8>, String> {
        if plan.is_empty() {
            return Err("Leerer Job — nichts zu lasern.".into());
        }
        Ok(build_packet(&self.build_job(plan), MAGIC))
    }
}

// --- Job-Bausteine (HW-verifiziert, nach Referenz) --------------------------

/// Preamble: Startmodus (Absolut), Rahmen-BBox und Diverses.
fn compile_preamble(minx: i32, miny: i32, maxx: i32, maxy: i32) -> Vec<u8> {
    let mut j = Vec::new();
    j.extend_from_slice(&[0xD8, 0x10, 0xE6, 0x01, 0xF0]); // Absolut
    j.extend_from_slice(&[0xF1, 0x02, 0x00]);
    j.extend_from_slice(&[0xD8, 0x00]);
    j.extend_from_slice(&[0xE7, 0x06]);
    j.extend_from_slice(&[0x00; 10]);
    j.extend_from_slice(&[0xE7, 0x38, 0x00]);
    j.extend_from_slice(&[0xE7, 0x03]);
    j.extend(encode_coord(minx));
    j.extend(encode_coord(miny));
    j.extend_from_slice(&[0xE7, 0x07]);
    j.extend(encode_coord(maxx));
    j.extend(encode_coord(maxy));
    j.extend_from_slice(&[0xE7, 0x50]);
    j.extend(encode_coord(minx));
    j.extend(encode_coord(miny));
    j.extend_from_slice(&[0xE7, 0x51]);
    j.extend(encode_coord(maxx));
    j.extend(encode_coord(maxy));
    j.extend_from_slice(&[0xE7, 0x04, 0x00, 0x01, 0x00, 0x01]);
    j.extend_from_slice(&[0x00; 10]);
    j.extend_from_slice(&[0xE7, 0x05, 0x00]);
    j
}

/// Layer-Config: pro Layer Speed/Power/Farbe/BBox, dann Abschluss-Blöcke.
fn compile_layer_config(layers: &[JobLayer], max_idx: u8) -> Vec<u8> {
    let mut j = Vec::new();
    for (k, jl) in layers.iter().enumerate() {
        let l = k as u8;
        let (lx0, ly0, lx1, ly1) = jl.bbox().unwrap_or((0.0, 0.0, 0.0, 0.0));
        j.extend_from_slice(&[0xC9, 0x04, l]);
        j.extend(encode_speed(jl.speed_mm_s));
        j.extend_from_slice(&[0xC6, 0x31, l]);
        j.extend(encode_power(jl.min_power_pct));
        j.extend_from_slice(&[0xC6, 0x32, l]);
        j.extend(encode_power(jl.power_pct));
        j.extend_from_slice(&[0xC6, 0x41, l]);
        j.extend(encode_power(jl.min_power_pct));
        j.extend_from_slice(&[0xC6, 0x42, l]);
        j.extend(encode_power(jl.power_pct));
        let [r, g, b] = jl.color;
        let bgr = ((b as u64) << 16) | ((g as u64) << 8) | (r as u64);
        j.extend_from_slice(&[0xCA, 0x06, l]);
        j.extend(encode_value(bgr, 5));
        j.extend_from_slice(&[0xCA, 0x41, l, 0x00]);
        j.extend_from_slice(&[0xE7, 0x52, l]);
        j.extend(encode_coord(mm_to_um(lx0)));
        j.extend(encode_coord(mm_to_um(ly0)));
        j.extend_from_slice(&[0xE7, 0x53, l]);
        j.extend(encode_coord(mm_to_um(lx1)));
        j.extend(encode_coord(mm_to_um(ly1)));
        j.extend_from_slice(&[0xE7, 0x61, l]);
        j.extend(encode_coord(mm_to_um(lx0)));
        j.extend(encode_coord(mm_to_um(ly0)));
        j.extend_from_slice(&[0xE7, 0x62, l]);
        j.extend(encode_coord(mm_to_um(lx1)));
        j.extend(encode_coord(mm_to_um(ly1)));
    }
    j.extend_from_slice(&[0xCA, 0x22, max_idx]);
    for code in [0x54u8, 0x55] {
        for l in 0..=max_idx {
            j.extend_from_slice(&[0xE7, code, l]);
            j.extend_from_slice(&[0x00; 5]);
        }
    }
    j
}

/// Settings-Block einer Ebene vor ihrer Geometrie (Layer-Select, Speed, Power).
fn write_settings_block(
    j: &mut Vec<u8>,
    is_cut: bool,
    l: u8,
    speed_mm_s: f64,
    min_power_pct: f64,
    power_pct: f64,
) {
    j.extend_from_slice(&[0xCA, 0x01, if is_cut { 0x00 } else { 0x01 }]);
    j.extend_from_slice(&[0xCA, 0x02, l]);
    j.extend_from_slice(&[0xCA, 0x01, 0x30]);
    j.extend_from_slice(&[0xCA, 0x01, 0x10]);
    j.extend_from_slice(&[0xCA, 0x01, 0x12]);
    j.extend_from_slice(&[0xC9, 0x02]);
    j.extend(encode_speed(speed_mm_s));
    let pw_min = encode_power(min_power_pct);
    let pw_max = encode_power(power_pct);
    j.extend_from_slice(&[0xC6, 0x01]);
    j.extend(pw_min.clone());
    j.extend_from_slice(&[0xC6, 0x02]);
    j.extend(pw_max.clone());
    j.extend_from_slice(&[0xC6, 0x21]);
    j.extend(pw_min);
    j.extend_from_slice(&[0xC6, 0x22]);
    j.extend(pw_max);
    j.extend_from_slice(&[0xCA, 0x03, 0x01]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use luxifer_core::{AppState, Geo};

    fn plan_one_rect() -> JobPlan {
        let mut st = AppState::new();
        st.add_shape(Geo::Rect {
            x: 1.0,
            y: 2.0,
            w: 10.0,
            h: 5.0,
        });
        JobPlan::from_shapes(&st.shapes, &st.layers)
    }

    #[test]
    fn job_beginnt_mit_preamble_und_endet_mit_eof() {
        let plan = plan_one_rect();
        let job = RuidaDriver.build_job(&plan);
        assert_eq!(&job[..2], &[0xD8, 0x10]); // Startmodus Absolut
        assert_eq!(*job.last().unwrap(), END_OF_FILE); // 0xD7
    }

    #[test]
    fn job_enthaelt_layer_config_und_geometrie() {
        let plan = plan_one_rect();
        let job = RuidaDriver.build_job(&plan);
        // Layer-Config: Speed-Opcode C9 04.
        assert!(job.windows(2).any(|w| w == [0xC9, 0x04]));
        // Geometrie: move (88) und cut (A8).
        assert!(job.contains(&0x88));
        assert!(job.contains(&0xA8));
    }

    #[test]
    fn compile_ist_geswizzelt_und_umkehrbar() {
        let plan = plan_one_rect();
        let pkt = RuidaDriver.compile(&plan, &[]).unwrap();
        let raw = RuidaDriver.build_job(&plan);
        assert_eq!(pkt.len(), raw.len() + 2);
        assert_eq!(unswizzle(&pkt[2..], MAGIC), raw);
    }

    #[test]
    fn leerer_job_ist_fehler() {
        let plan = JobPlan {
            layers: vec![],
            bbox: None,
        };
        assert!(RuidaDriver.compile(&plan, &[]).is_err());
    }
}
