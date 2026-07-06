//! Ruida-Treiber: übersetzt den geräteunabhängigen [`JobPlan`] in einen
//! Ruida-Binärjob (RDC6445G).
//!
//! Kennt nur `luxifer-core` (ADR 0001). Implementiert den geometrischen Kern:
//! pro Layer Speed/Power, dann Move/Cut-Befehle in µm, abschließend Swizzle +
//! Checksum-Paket.
//!
//! HINWEIS (Ausbaustufe): Die vollständige Ruida-Präambel, Multi-Layer-Config
//! und der Trailer, die eine reale Maschine zum Ausführen erwartet, sind hier
//! noch NICHT enthalten. Der erzeugte Bytestrom ist protokoll-korrekt kodiert
//! (an HW verifizierte Kodierung aus der Referenz), aber noch kein
//! sende-fertiger Job. Siehe docs/referenz für den vollständigen Compiler.

pub mod protocol;

use luxifer_core::{JobPlan, Layer, LayerWork, MachineDriver};
use protocol::*;

/// Der Ruida-Treiber.
#[derive(Debug, Clone, Default)]
pub struct RuidaDriver;

impl RuidaDriver {
    /// Baut den rohen (ungeswizzelten) Befehlsstrom aus dem Plan.
    /// Nützlich für Tests/Analyse.
    pub fn raw_commands(&self, plan: &JobPlan) -> Vec<u8> {
        let mut out = Vec::new();
        for jl in &plan.layers {
            out.extend(cmd_set_speed(jl.speed_mm_s));
            out.extend(cmd_set_power_min(jl.min_power_pct));
            out.extend(cmd_set_power_max(jl.power_pct));

            let passes = jl.passes.max(1);
            for _ in 0..passes {
                match &jl.work {
                    LayerWork::Cut { paths } => {
                        for path in paths {
                            if path.points.is_empty() {
                                continue;
                            }
                            let (x0, y0) = path.points[0];
                            out.extend(cmd_move_abs(mm_to_um(x0), mm_to_um(y0)));
                            for &(x, y) in &path.points[1..] {
                                out.extend(cmd_cut_abs(mm_to_um(x), mm_to_um(y)));
                            }
                            if path.closed {
                                out.extend(cmd_cut_abs(mm_to_um(x0), mm_to_um(y0)));
                            }
                        }
                    }
                    LayerWork::Fill { segments } => {
                        for seg in segments {
                            out.extend(cmd_move_abs(mm_to_um(seg.x0), mm_to_um(seg.y)));
                            out.extend(cmd_cut_abs(mm_to_um(seg.x1), mm_to_um(seg.y)));
                        }
                    }
                }
            }
        }
        out.extend(cmd_stop());
        out
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
        let raw = self.raw_commands(plan);
        Ok(build_packet(&raw, MAGIC))
    }
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
    fn raw_beginnt_mit_speed_und_enthaelt_move_und_cut() {
        let plan = plan_one_rect();
        let raw = RuidaDriver.raw_commands(&plan);
        assert_eq!(raw[0], 0xC9); // set_speed
        assert!(raw.contains(&0x88)); // move_abs (Startpunkt)
        assert!(raw.contains(&0xA8)); // cut_abs (Kanten)
        assert_eq!(*raw.last().unwrap(), 0x01); // cmd_stop = D8 01
    }

    #[test]
    fn compile_liefert_geswizzeltes_paket() {
        let plan = plan_one_rect();
        let pkt = RuidaDriver.compile(&plan, &[]).unwrap();
        // Paket = 2 Byte Checksum + geswizzelte Nutzdaten.
        let raw = RuidaDriver.raw_commands(&plan);
        assert_eq!(pkt.len(), raw.len() + 2);
        // Nutzdaten müssen sich zurück-entschlüsseln lassen.
        let back = unswizzle(&pkt[2..], MAGIC);
        assert_eq!(back, raw);
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
