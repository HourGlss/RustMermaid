//! Diagram types and parsing

pub mod architecture;
pub mod block;
pub mod c4;
pub mod class;
pub(crate) mod direction;
pub mod er;
pub mod flowchart;
pub mod gantt;
pub mod git;
pub mod info;
pub mod journey;
pub mod kanban;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant;
pub mod radar;
pub mod requirement;
pub mod sankey;
pub mod sequence;
pub mod state;
pub mod timeline;
pub mod treemap;
pub mod xychart;

mod detect;
pub mod directive;

pub use detect::{detect_type, DiagramType};
pub use directive::{detect_init, remove_directives, DiagramConfig};

use crate::error::Result;

/// A parsed mermaid diagram
#[derive(Debug, Clone)]
pub enum Diagram {
    Architecture(architecture::ArchitectureDb),
    Block(block::BlockDb),
    C4(c4::C4Db),
    Class(class::ClassDb),
    Er(er::ErDb),
    Flowchart(flowchart::FlowchartDb),
    Gantt(gantt::GanttDb),
    Git(git::GitGraphDb),
    Info(info::InfoDb),
    Journey(journey::JourneyDb),
    Kanban(kanban::KanbanDb),
    Mindmap(mindmap::MindmapDb),
    Packet(packet::PacketDb),
    Pie(pie::PieDb),
    Quadrant(quadrant::QuadrantDb),
    Radar(radar::RadarDb),
    Requirement(requirement::RequirementDb),
    Sankey(sankey::SankeyDb),
    Sequence(sequence::SequenceDb),
    State(state::StateDb),
    Timeline(timeline::TimelineDb),
    Treemap(treemap::TreemapDb),
    XyChart(xychart::XYChartDb),
}

/// Parse a diagram of a specific type
pub fn parse(diagram_type: DiagramType, input: &str) -> Result<Diagram> {
    parse_common_diagram(diagram_type, input)
        .unwrap_or_else(|| parse_extended_diagram(diagram_type, input))
}

fn parse_common_diagram(diagram_type: DiagramType, input: &str) -> Option<Result<Diagram>> {
    match diagram_type {
        DiagramType::Architecture => Some(
            architecture::parse(input)
                .map(Diagram::Architecture)
                .map_err(Into::into),
        ),
        DiagramType::Block => Some(block::parse(input).map(Diagram::Block).map_err(Into::into)),
        DiagramType::C4 => Some(c4::parse(input).map(Diagram::C4).map_err(Into::into)),
        DiagramType::Class => Some(class::parse(input).map(Diagram::Class).map_err(Into::into)),
        DiagramType::Er => Some(er::parse(input).map(Diagram::Er)),
        DiagramType::Flowchart => Some(flowchart::parse(input).map(Diagram::Flowchart)),
        DiagramType::Gantt => Some(gantt::parse(input).map(Diagram::Gantt).map_err(Into::into)),
        DiagramType::Git => Some(git::parse(input).map(Diagram::Git).map_err(Into::into)),
        DiagramType::Info => Some(info::parse(input).map(Diagram::Info)),
        DiagramType::Journey => Some(
            journey::parse(input)
                .map(Diagram::Journey)
                .map_err(Into::into),
        ),
        DiagramType::Kanban => Some(
            kanban::parse(input)
                .map(Diagram::Kanban)
                .map_err(Into::into),
        ),
        _ => None,
    }
}

fn parse_extended_diagram(diagram_type: DiagramType, input: &str) -> Result<Diagram> {
    match diagram_type {
        DiagramType::Mindmap => mindmap::parse(input).map(Diagram::Mindmap),
        DiagramType::Packet => packet::parse(input)
            .map(Diagram::Packet)
            .map_err(Into::into),
        DiagramType::Pie => pie::parse(input).map(Diagram::Pie),
        DiagramType::Quadrant => quadrant::parse(input)
            .map(Diagram::Quadrant)
            .map_err(Into::into),
        DiagramType::Radar => radar::parse(input).map(Diagram::Radar).map_err(Into::into),
        DiagramType::Requirement => requirement::parse(input)
            .map(Diagram::Requirement)
            .map_err(Into::into),
        DiagramType::Sankey => sankey::parse(input)
            .map(Diagram::Sankey)
            .map_err(Into::into),
        DiagramType::Sequence => sequence::parse(input)
            .map(Diagram::Sequence)
            .map_err(Into::into),
        DiagramType::State => state::parse(input).map(Diagram::State).map_err(Into::into),
        DiagramType::Timeline => timeline::parse(input)
            .map(Diagram::Timeline)
            .map_err(Into::into),
        DiagramType::Treemap => treemap::parse(input)
            .map(Diagram::Treemap)
            .map_err(Into::into),
        DiagramType::XyChart => xychart::parse(input)
            .map(Diagram::XyChart)
            .map_err(Into::into),
        _ => unreachable!("common diagram types are handled before extended parsing"),
    }
}
