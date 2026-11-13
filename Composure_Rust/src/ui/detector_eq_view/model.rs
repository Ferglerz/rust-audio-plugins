use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Node {
    Band(usize),
    HighPass,
    LowPass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Field {
    Frequency,
    Gain,
    Q,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Target {
    pub node: Node,
    pub field: Field,
}

pub(super) struct NodeDrag {
    pub node: Node,
    pub fields: Vec<Field>,
    pub origin: (f32, f32),
    pub frequency: f32,
    pub gain: f32,
    pub q: f32,
}

pub(super) struct ReadoutPress {
    pub press: ValuePress<Target>,
    pub start: f32,
    pub active: bool,
}

pub(super) fn node_fields(node: Node, shape: EqShape) -> Vec<Field> {
    if matches!(node, Node::Band(_)) {
        if shape.has_gain() {
            vec![Field::Frequency, Field::Gain, Field::Q]
        } else {
            vec![Field::Frequency, Field::Q]
        }
    } else {
        vec![Field::Frequency]
    }
}

pub(super) fn node_color(node: Node) -> nih_plug_vizia::vizia::vg::Color {
    COLORS[match node {
        Node::Band(i) => i,
        Node::HighPass => 6,
        Node::LowPass => 7,
    } % COLORS.len()]
}

pub(super) fn set_once<P: Param>(cx: &mut EventContext, param: &P, normalized: f32) {
    pleasant_ui::param::set_normalized_once(cx, param.as_ptr(), normalized);
}

impl DetectorEqView {
    pub(super) fn settings(&self, node: Node) -> BandSettings {
        match node {
            Node::Band(i) => self.params.detector_eq[i].settings(),
            Node::HighPass | Node::LowPass => BandSettings {
                shape: if node == Node::HighPass {
                    EqShape::LowCut
                } else {
                    EqShape::HighCut
                },
                order: 2,
                frequency_hz: self
                    .param(Target {
                        node,
                        field: Field::Frequency,
                    })
                    .value() as f64,
                gain_db: 0.0,
                q: std::f64::consts::FRAC_1_SQRT_2,
                ..BandSettings::default()
            },
        }
    }

    pub(super) fn nodes(&self) -> Vec<Node> {
        let mut nodes: Vec<_> = self
            .params
            .detector_eq
            .iter()
            .enumerate()
            .filter(|(_, band)| band.active.value())
            .map(|(i, _)| Node::Band(i))
            .collect();
        if self.params.hp_freq.value() > 0.0 {
            nodes.push(Node::HighPass);
        }
        if self.params.lp_freq.value() > 0.0 {
            nodes.push(Node::LowPass);
        }
        nodes
    }

    pub(super) fn hit(&self, p: (f32, f32)) -> Option<Node> {
        let mut nodes = self.nodes();
        // The selected node stays on top when nodes overlap.
        nodes.sort_by_key(|node| Some(*node) == self.selected);
        nodes.into_iter().rev().find(|node| {
            let b = self.settings(*node);
            (frequency_x(b.frequency_hz as f32) - p.0).hypot(
                gain_y(if b.shape.has_gain() {
                    b.gain_db as f32
                } else {
                    0.0
                }) - p.1,
            ) <= 16.0
        })
    }

    pub(super) fn param(&self, target: Target) -> &FloatParam {
        match target.node {
            Node::HighPass => &self.params.hp_freq,
            Node::LowPass => &self.params.lp_freq,
            Node::Band(i) => match target.field {
                Field::Frequency => &self.params.detector_eq[i].freq,
                Field::Gain => &self.params.detector_eq[i].gain,
                Field::Q => &self.params.detector_eq[i].q,
            },
        }
    }

    pub(super) fn fields(&self, node: Node) -> Vec<Field> {
        node_fields(node, self.settings(node).shape)
    }

    pub(super) fn value_hit(&self, p: (f32, f32)) -> Option<Target> {
        let node = self.selected?;
        self.fields(node)
            .into_iter()
            .find(|field| point_in_rect(p.0, p.1, field_rect(*field, &self.fields(node))))
            .map(|field| Target { node, field })
    }
}
