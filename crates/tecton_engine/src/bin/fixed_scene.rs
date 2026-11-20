//! Convert a validated fixed-layout scene JSON into native Tecton SVG.
use std::{
    env, fs,
    io::{self, Write},
};

use anyhow::{bail, Result};
use tecton_render::{fixed_scene::FixedScene, SvgBackend};

fn main() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let Some(input) = args.next() else {
        bail!("usage: fixed_scene INPUT.json > OUTPUT.svg");
    };
    if args.next().is_some() {
        bail!("usage: fixed_scene INPUT.json > OUTPUT.svg");
    }
    let scene: FixedScene = serde_json::from_slice(&fs::read(input)?)?;
    io::stdout()
        .lock()
        .write_all(SvgBackend::render_fixed_scene(&scene)?.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // Synthetic content; no customer diagram or private labels are versioned.
    fn fixture() -> Value {
        serde_json::from_str(r##"{
            "canvas":[200,130],
            "nodes":[{"id":"group","rect":[2,2,196,126],"fill":"#ededed","border":"#aaaaaa","border_width":1,"border_radius":3,"border_dash":[4,2],
              "children":[
                {"id":"a","rect":[10,15,30,20],"fill":"#abcdef","border":"none","border_width":0,"border_radius":2,
                 "text_runs":[{"text":"甲 <&>","rect":[12,19,20,10],"font_size":12,"color":"#123456","baseline_y":29,"font_weight":660}]},
                {"id":"b","rect":[10,70,30,20],"fill":"#cdefab","border":"none","border_width":0,"border_radius":0},
                {"id":"c","rect":[100,40,30,25],"fill":"#fedcba","border":"none","border_width":0,"border_radius":0}
              ]}],
            "edges":[
                {"id":"a-leg","points":[[40,25],[62,25],[62,50]],"color":"#223344","width":2,"kind":"solid","arrow_start":false,"arrow_end":false},
                {"id":"b-leg","points":[[40,80],[62,80],[62,50]],"color":"#223344","width":2,"kind":"solid","arrow_start":false,"arrow_end":false},
                {"id":"trunk","points":[[62,50],[100,50]],"color":"#223344","width":2,"kind":"solid","arrow_start":false,"arrow_end":true},
                {"id":"reverse","points":[[100,60],[78,60],[78,31],[40,31]],"color":"#445566","width":2,"kind":"dashed","dash":[7,3],"arrow_start":true,"arrow_end":true}
            ],
            "logical_edges":[
                {"id":"a-to-c","source":"a","target":"c","routes":["a-leg","trunk"],"bidirectional":false},
                {"id":"b-to-c","source":"b","target":"c","routes":["b-leg","trunk"],"bidirectional":false},
                {"id":"c-to-a","source":"c","target":"a","routes":["reverse"],"bidirectional":true}
            ],
            "annotations":[{"text":"Label","rect":[1,9,20,10],"font_size":12,"color":"#123456","baseline_y":11,
             "font_weight":780,"font_file":"intentionally-not-read"}]
        }"##)
        .unwrap()
    }

    fn render(value: Value) -> Result<String> {
        let scene: FixedScene = serde_json::from_value(value)?;
        Ok(SvgBackend::render_fixed_scene(&scene)?)
    }

    #[test]
    fn fixed_scene_preserves_geometry_layers_and_text() {
        let svg = render(fixture()).unwrap();
        assert!(svg.contains(r#"viewBox="0 0 200 130""#));
        assert_eq!(svg.matches("<polyline ").count(), 4);
        assert_eq!(svg.matches(r#"points="62, 50 100, 50""#).count(), 1);
        assert_eq!(svg.matches(" marker-start=").count(), 1);
        assert_eq!(svg.matches(" marker-end=").count(), 2);
        assert_eq!(svg.matches(r#"orient="auto-start-reverse""#).count(), 2);
        assert!(svg.contains(r#"stroke-dasharray="4 2""#));
        assert!(svg.contains(r#"stroke-dasharray="7 3""#));
        assert!(svg
            .contains(r##"<text x="12" y="29" fill="#123456" font-size="12" font-weight="660""##));
        assert!(svg.contains("甲 &lt;&amp;&gt;"));
        assert!(svg.contains(r#"font-weight="780""#));
        assert!(!svg.contains("intentionally-not-read"));
        assert!(svg.find("#ededed") < svg.find("<polyline"));
        assert!(svg.find("<polyline") < svg.find("#abcdef"));
        assert!(svg.find("#abcdef") < svg.find("<text"));
    }

    #[test]
    fn invalid_route_junction_and_anchor_are_rejected() {
        let mut scene = fixture();
        scene["edges"][0]["arrow_end"] = true.into();
        assert!(render(scene)
            .unwrap_err()
            .to_string()
            .contains("junction arrow"));

        let mut scene = fixture();
        scene["edges"][0]["points"][2][1] = 49.into();
        assert!(render(scene)
            .unwrap_err()
            .to_string()
            .contains("disconnected"));

        let mut scene = fixture();
        scene["edges"][0]["points"][0][0] = 41.into();
        assert!(render(scene)
            .unwrap_err()
            .to_string()
            .contains("off its node boundary"));
    }

    #[test]
    fn invalid_fallback_label_is_rejected() {
        let mut scene = fixture();
        scene["nodes"][0]["children"][1]["label"] = "illegal\0control".into();
        assert!(render(scene)
            .unwrap_err()
            .to_string()
            .contains("invalid fixed node label"));
    }

    #[test]
    fn unsafe_paints_and_duplicate_visual_ids_are_rejected() {
        let mut scene = fixture();
        scene["annotations"][0]["color"] = "url(https://example.invalid)".into();
        assert!(render(scene).is_err());

        let mut scene = fixture();
        scene["edges"][2]["id"] = "reverse".into();
        assert!(render(scene)
            .unwrap_err()
            .to_string()
            .contains("duplicate visual route ID"));
    }
}
