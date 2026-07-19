//! Geräteautoritative Bewegungsspur zwischen JobPlan und Serialisierung.

use crate::geometry::Pt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionKind {
    Travel,
    Cut,
    Fill,
    Raster,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionMove {
    /// Ideale Geometrie vor maschinenspezifischer Kompensation.
    pub ideal_from: Pt,
    pub ideal_to: Pt,
    /// Tatsächlich ausgeführte Geometrie nach Offset/Quantisierung.
    pub from: Pt,
    pub to: Pt,
    pub kind: ExecutionKind,
    pub layer_id: usize,
    pub seq: u32,
}

impl ExecutionMove {
    pub fn laser_on(self) -> bool {
        self.kind != ExecutionKind::Travel
    }

    pub fn len_mm(self) -> f64 {
        (self.to.0 - self.from.0).hypot(self.to.1 - self.from.1)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecutionTrace {
    pub moves: Vec<ExecutionMove>,
    pub scan_offset_active: bool,
}

impl ExecutionTrace {
    pub fn laser_len_mm(&self) -> f64 {
        self.moves
            .iter()
            .filter(|m| m.laser_on())
            .map(|m| m.len_mm())
            .sum()
    }

    pub fn travel_len_mm(&self) -> f64 {
        self.moves
            .iter()
            .filter(|m| !m.laser_on())
            .map(|m| m.len_mm())
            .sum()
    }

    pub fn total_len_mm(&self) -> f64 {
        self.moves.iter().map(|m| m.len_mm()).sum()
    }
}

pub struct TraceBuilder {
    trace: ExecutionTrace,
    head: Option<Pt>,
}

impl TraceBuilder {
    pub fn new(scan_offset_active: bool) -> Self {
        Self {
            trace: ExecutionTrace {
                moves: Vec::new(),
                scan_offset_active,
            },
            head: None,
        }
    }

    pub fn set_head(&mut self, head: Pt) {
        self.head = Some(head);
    }

    pub fn travel_to(&mut self, to: Pt, layer_id: usize) {
        if let Some(from) = self.head {
            if from != to {
                self.push(from, to, from, to, ExecutionKind::Travel, layer_id);
            }
        }
        self.head = Some(to);
    }

    pub fn work(
        &mut self,
        ideal_from: Pt,
        ideal_to: Pt,
        from: Pt,
        to: Pt,
        kind: ExecutionKind,
        layer_id: usize,
    ) {
        if let Some(head) = self.head {
            if head != from {
                self.push(head, from, head, from, ExecutionKind::Travel, layer_id);
            }
        }
        self.push(ideal_from, ideal_to, from, to, kind, layer_id);
        self.head = Some(to);
    }

    pub fn finish(self) -> ExecutionTrace {
        self.trace
    }

    fn push(
        &mut self,
        ideal_from: Pt,
        ideal_to: Pt,
        from: Pt,
        to: Pt,
        kind: ExecutionKind,
        layer_id: usize,
    ) {
        let seq = self.trace.moves.len() as u32;
        self.trace.moves.push(ExecutionMove {
            ideal_from,
            ideal_to,
            from,
            to,
            kind,
            layer_id,
            seq,
        });
    }
}
