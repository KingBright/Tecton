use taffy::{prelude::*, TaffyTree};
use tecton_types::{
    geometry::{Rect, Size},
    id::NodeId,
    style::{FlexDirection, Style as TectonStyle},
};

use crate::layout_context::{LayoutNode, LayoutResult};

/// Adapter for Taffy flexbox layout engine
///
/// This wraps Taffy and provides integration with Tecton types
pub struct TaffyAdapter {
    tree: TaffyTree,
}

impl Default for TaffyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyAdapter {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
        }
    }

    /// Convert Tecton style to Taffy style
    fn convert_style(&self, style: &TectonStyle) -> taffy::Style {
        use taffy::style_helpers::length;

        taffy::Style {
            display: Display::Flex,
            flex_direction: match style.flex_direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::Column => taffy::FlexDirection::Column,
                FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            padding: taffy::Rect {
                left: length(style.padding.left()),
                right: length(style.padding.right()),
                top: length(style.padding.top()),
                bottom: length(style.padding.bottom()),
            },
            margin: taffy::Rect {
                left: length(style.margin.left()),
                right: length(style.margin.right()),
                top: length(style.margin.top()),
                bottom: length(style.margin.bottom()),
            },
            gap: taffy::Size {
                width: length(style.gap()),
                height: length(style.gap()),
            },
            ..Default::default()
        }
    }

    /// Create a simple flexbox layout
    pub fn layout_flexbox(
        &mut self,
        bounds: Rect,
        items: &[(NodeId, Size, TectonStyle)],
        direction: FlexDirection,
        gap: f32,
    ) -> Result<Vec<(NodeId, Rect)>, taffy::TaffyError> {
        if items.is_empty() {
            return Ok(Vec::new());
        }

        // Create leaf nodes for each item
        let mut taffy_ids = Vec::new();
        let mut id_mapping = Vec::new();

        for (node_id, size, style) in items {
            let mut taffy_style = self.convert_style(style);
            // Set fixed size if provided
            if size.width() > 0.0 && size.height() > 0.0 {
                taffy_style.size = taffy::Size {
                    width: taffy::Dimension::Length(size.width()),
                    height: taffy::Dimension::Length(size.height()),
                };
            }

            let taffy_id = self.tree.new_leaf(taffy_style)?;
            taffy_ids.push(taffy_id);
            id_mapping.push((*node_id, taffy_id));
        }

        // Create container with flex direction
        let container_style = taffy::Style {
            display: Display::Flex,
            flex_direction: match direction {
                FlexDirection::Row => taffy::FlexDirection::Row,
                FlexDirection::Column => taffy::FlexDirection::Column,
                FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
                FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
            },
            gap: taffy::Size {
                width: length(gap),
                height: length(gap),
            },
            size: taffy::Size {
                width: taffy::Dimension::Length(bounds.width()),
                height: taffy::Dimension::Length(bounds.height()),
            },
            ..Default::default()
        };

        let container_id = self.tree.new_with_children(container_style, &taffy_ids)?;

        // Compute layout
        let available_size = taffy::Size {
            width: bounds.width().into(),
            height: bounds.height().into(),
        };
        self.tree.compute_layout(container_id, available_size)?;

        // Extract results
        let mut results = Vec::new();
        for (node_id, taffy_id) in id_mapping {
            let layout = self.tree.layout(taffy_id)?;
            let rect = Rect::new(
                bounds.x() + layout.location.x,
                bounds.y() + layout.location.y,
                layout.size.width,
                layout.size.height,
            );
            results.push((node_id, rect));
        }

        Ok(results)
    }
}

/// Layout a container node using Taffy
pub fn layout_container(
    bounds: Rect,
    children: &[(NodeId, Size, TectonStyle)],
    container_style: &TectonStyle,
) -> Result<Vec<(NodeId, Rect)>, Box<dyn std::error::Error>> {
    let mut adapter = TaffyAdapter::new();

    // Create child nodes
    let mut taffy_children = Vec::new();
    let mut id_map: std::collections::HashMap<taffy::NodeId, NodeId> =
        std::collections::HashMap::new();

    for (node_id, size, style) in children {
        let taffy_style = convert_tecton_style(style, Some(*size));
        let taffy_id = adapter.tree.new_leaf(taffy_style)?;
        taffy_children.push(taffy_id);
        id_map.insert(taffy_id, *node_id);
    }

    // Create container
    let container_taffy_style = taffy::Style {
        display: Display::Flex,
        flex_direction: match container_style.flex_direction {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        },
        padding: taffy::Rect {
            left: taffy::style_helpers::length(container_style.padding.left()),
            right: taffy::style_helpers::length(container_style.padding.right()),
            top: taffy::style_helpers::length(container_style.padding.top()),
            bottom: taffy::style_helpers::length(container_style.padding.bottom()),
        },
        gap: taffy::Size {
            width: taffy::style_helpers::length(container_style.gap()),
            height: taffy::style_helpers::length(container_style.gap()),
        },
        size: taffy::Size {
            width: taffy::Dimension::Length(bounds.width()),
            height: taffy::Dimension::Length(bounds.height()),
        },
        ..Default::default()
    };

    let container_id = adapter
        .tree
        .new_with_children(container_taffy_style, &taffy_children)?;

    // Compute layout
    let available_size = taffy::Size {
        width: bounds.width().into(),
        height: bounds.height().into(),
    };
    adapter.tree.compute_layout(container_id, available_size)?;

    // Extract results
    let mut results = Vec::new();
    for (taffy_id, node_id) in id_map {
        let layout = adapter.tree.layout(taffy_id)?;
        let rect = Rect::new(
            bounds.x() + layout.location.x,
            bounds.y() + layout.location.y,
            layout.size.width,
            layout.size.height,
        );
        results.push((node_id, rect));
    }

    Ok(results)
}

/// Convert Tecton style to Taffy style
fn convert_tecton_style(style: &TectonStyle, fixed_size: Option<Size>) -> taffy::Style {
    use taffy::style_helpers::length;

    let mut taffy_style = taffy::Style {
        display: Display::Flex,
        flex_direction: match style.flex_direction {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        },
        padding: taffy::Rect {
            left: length(style.padding.left()),
            right: length(style.padding.right()),
            top: length(style.padding.top()),
            bottom: length(style.padding.bottom()),
        },
        margin: taffy::Rect {
            left: length(style.margin.left()),
            right: length(style.margin.right()),
            top: length(style.margin.top()),
            bottom: length(style.margin.bottom()),
        },
        gap: taffy::Size {
            width: length(style.gap()),
            height: length(style.gap()),
        },
        ..Default::default()
    };

    if let Some(size) = fixed_size {
        if size.width() > 0.0 {
            taffy_style.size.width = taffy::Dimension::Length(size.width());
        }
        if size.height() > 0.0 {
            taffy_style.size.height = taffy::Dimension::Length(size.height());
        }
    }

    taffy_style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taffy_adapter_basic() {
        let mut adapter = TaffyAdapter::new();

        let items: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Row, 10.0);

        assert!(result.is_ok());
        let layouts = result.unwrap();
        assert_eq!(layouts.len(), 2);
    }

    // ========== New tests for comprehensive coverage ==========

    #[test]
    fn test_taffy_adapter_default() {
        let adapter = TaffyAdapter::default();
        // Just verify it can be created
        assert!(true);
    }

    #[test]
    fn test_taffy_adapter_empty_items() {
        let mut adapter = TaffyAdapter::new();
        let items: Vec<(NodeId, Size, TectonStyle)> = vec![];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Row, 10.0);

        assert!(result.is_ok());
        let layouts = result.unwrap();
        assert!(layouts.is_empty());
    }

    #[test]
    fn test_taffy_adapter_row_layout() {
        let mut adapter = TaffyAdapter::new();

        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let items: Vec<(NodeId, Size, TectonStyle)> = vec![
            (id1, Size::new(50.0, 50.0), TectonStyle::default()),
            (id2, Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Row, 0.0);

        assert!(result.is_ok());
        let layouts = result.unwrap();

        // In row layout, items should be side by side
        let layout1 = layouts.iter().find(|(id, _)| *id == id1).unwrap();
        let layout2 = layouts.iter().find(|(id, _)| *id == id2).unwrap();

        // Both should start at y = 0
        assert_eq!(layout1.1.y(), 0.0);
        assert_eq!(layout2.1.y(), 0.0);

        // Second item should be to the right of first
        assert!(layout2.1.x() >= layout1.1.right());
    }

    #[test]
    fn test_taffy_adapter_column_layout() {
        let mut adapter = TaffyAdapter::new();

        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let items: Vec<(NodeId, Size, TectonStyle)> = vec![
            (id1, Size::new(50.0, 50.0), TectonStyle::default()),
            (id2, Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 100.0, 200.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Column, 0.0);

        assert!(result.is_ok());
        let layouts = result.unwrap();

        // In column layout, items should be stacked vertically
        let layout1 = layouts.iter().find(|(id, _)| *id == id1).unwrap();
        let layout2 = layouts.iter().find(|(id, _)| *id == id2).unwrap();

        // Both should start at x = 0
        assert_eq!(layout1.1.x(), 0.0);
        assert_eq!(layout2.1.x(), 0.0);

        // Second item should be below first
        assert!(layout2.1.y() >= layout1.1.bottom());
    }

    #[test]
    fn test_taffy_adapter_with_gap() {
        let mut adapter = TaffyAdapter::new();

        let items: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Row, 20.0);

        assert!(result.is_ok());
        let layouts = result.unwrap();
        assert_eq!(layouts.len(), 2);
    }

    #[test]
    fn test_layout_container_basic() {
        let children: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let container_style = TectonStyle::default();

        let result = layout_container(bounds, &children, &container_style);

        assert!(result.is_ok());
        let layouts = result.unwrap();
        assert_eq!(layouts.len(), 2);
    }

    #[test]
    fn test_layout_container_with_padding() {
        let children: Vec<(NodeId, Size, TectonStyle)> =
            vec![(NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default())];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let container_style = TectonStyle {
            padding: tecton_types::style::Padding::uniform(10.0),
            ..Default::default()
        };

        let result = layout_container(bounds, &children, &container_style);

        assert!(result.is_ok());
    }

    #[test]
    fn test_layout_container_with_gap() {
        let children: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let container_style = TectonStyle {
            gap: 15.0.into(),
            ..Default::default()
        };

        let result = layout_container(bounds, &children, &container_style);

        assert!(result.is_ok());
    }

    #[test]
    fn test_layout_container_row_reverse() {
        let children: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let container_style = TectonStyle {
            flex_direction: FlexDirection::RowReverse,
            ..Default::default()
        };

        let result = layout_container(bounds, &children, &container_style);

        assert!(result.is_ok());
    }

    #[test]
    fn test_layout_container_column_reverse() {
        let children: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
            (NodeId::new(), Size::new(50.0, 50.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 100.0, 200.0);
        let container_style = TectonStyle {
            flex_direction: FlexDirection::ColumnReverse,
            ..Default::default()
        };

        let result = layout_container(bounds, &children, &container_style);

        assert!(result.is_ok());
    }

    #[test]
    fn test_taffy_adapter_item_without_fixed_size() {
        let mut adapter = TaffyAdapter::new();

        let items: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(0.0, 0.0), TectonStyle::default()),
            (NodeId::new(), Size::new(0.0, 0.0), TectonStyle::default()),
        ];

        let bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Row, 0.0);

        assert!(result.is_ok());
    }

    #[test]
    fn test_taffy_adapter_nested_containers() {
        // This tests that we can handle nested container scenarios
        // by calling layout_container multiple times
        let inner_children: Vec<(NodeId, Size, TectonStyle)> = vec![
            (NodeId::new(), Size::new(25.0, 25.0), TectonStyle::default()),
            (NodeId::new(), Size::new(25.0, 25.0), TectonStyle::default()),
        ];

        let inner_bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let inner_result = layout_container(inner_bounds, &inner_children, &TectonStyle::default());

        assert!(inner_result.is_ok());
        assert_eq!(inner_result.unwrap().len(), 2);
    }

    #[test]
    fn test_taffy_adapter_many_items() {
        let mut adapter = TaffyAdapter::new();

        let items: Vec<(NodeId, Size, TectonStyle)> = (0..10)
            .map(|_| (NodeId::new(), Size::new(30.0, 30.0), TectonStyle::default()))
            .collect();

        let bounds = Rect::new(0.0, 0.0, 500.0, 100.0);
        let result = adapter.layout_flexbox(bounds, &items, FlexDirection::Row, 5.0);

        assert!(result.is_ok());
        let layouts = result.unwrap();
        assert_eq!(layouts.len(), 10);
    }
}
