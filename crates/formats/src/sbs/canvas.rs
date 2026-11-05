use super::sbs_writer::SbsWriter;
use super::xml::{format_float, xml_escape};
use materialcraft_core::{Graph, linear_to_srgb};
use std::fmt::Write as _;

pub(super) fn write_canvas_items(output: &mut String, graph: &Graph, writer: &mut SbsWriter) {
    output.push_str("<GUIObjects>");

    for frame in &graph.frames {
        let [x, y, width, height] = frame.rect.map(format_float);
        let [red, green, blue] = [0, 1, 2].map(|channel| format_float(linear_to_srgb(frame.color[channel])));
        let _ = write!(
            output,
            "<GUIObject><type v=\"COMMENT\"/><GUILayout><gpos v=\"{x} {y} -100\"/><size v=\"{width} {height}\"/></GUILayout><GUIName v=\"{}\"/><uid v=\"{}\"/><title v=\"{}\"/><frameColor v=\"{red} {green} {blue} {}\"/><isTitleVisible v=\"{}\"/><isFrameVisible v=\"1\"/></GUIObject>",
            xml_escape(&frame.description),
            writer.next_uid(),
            xml_escape(&frame.title),
            format_float(frame.color[3]),
            u8::from(frame.title_visible)
        );
    }

    for comment in &graph.comments {
        let [x, y] = comment.pos.map(format_float);
        let [width, height] = comment.size.map(format_float);
        let _ = write!(
            output,
            "<GUIObject><type v=\"COMMENT\"/><GUILayout><gpos v=\"{x} {y} 0\"/><size v=\"{width} {height}\"/></GUILayout><GUIName v=\"{}\"/><uid v=\"{}\"/><isFrameVisible v=\"0\"/></GUIObject>",
            xml_escape(&comment.text),
            writer.next_uid()
        );
    }

    for pin in &graph.pins {
        let [x, y] = pin.pos.map(format_float);
        let _ = write!(
            output,
            "<GUIObject><type v=\"PIN\"/><GUILayout><gpos v=\"{x} {y} 0\"/></GUILayout><GUIName v=\"{}\"/><uid v=\"{}\"/></GUIObject>",
            xml_escape(&pin.name),
            writer.next_uid()
        );
    }

    output.push_str("</GUIObjects>");
}
