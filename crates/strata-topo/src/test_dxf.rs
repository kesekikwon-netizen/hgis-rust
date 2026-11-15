//! Small DXF drawings for the tests, in the form of a received sheet (AC1015, code page 949).

/// One entity of a test drawing.
pub(crate) enum Entity<'a> {
    /// A polyline on a CAD layer, with a height when it has one.
    Line(&'a str, Option<f64>, &'a [[f64; 2]]),
    /// A text on a CAD layer.
    Text(&'a str, [f64; 2]),
}

/// The text of a drawing with these entities.
pub(crate) fn drawing(entities: &[Entity<'_>]) -> String {
    let mut groups: Vec<(String, String)> = [("0", "SECTION"), ("2", "HEADER"), ("9", "$ACADVER"), ("1", "AC1015"), ("9", "$DWGCODEPAGE"), ("3", "ANSI_949"), ("0", "ENDSEC"), ("0", "SECTION"), ("2", "ENTITIES")]
        .iter()
        .map(|(code, value)| ((*code).to_owned(), (*value).to_owned()))
        .collect();
    let mut add = |code: &str, value: String| groups.push((code.to_owned(), value));
    for entity in entities {
        match entity {
            Entity::Line(layer, height, points) => {
                add("0", "LWPOLYLINE".to_owned());
                add("8", (*layer).to_owned());
                if let Some(z) = height {
                    add("38", format!("{z}"));
                }
                add("90", points.len().to_string());
                add("70", "0".to_owned());
                for point in *points {
                    add("10", format!("{}", point[0]));
                    add("20", format!("{}", point[1]));
                }
            }
            Entity::Text(layer, at) => {
                add("0", "TEXT".to_owned());
                add("8", (*layer).to_owned());
                add("10", format!("{}", at[0]));
                add("20", format!("{}", at[1]));
                add("40", "5.0".to_owned());
                add("1", "name".to_owned());
            }
        }
    }
    add("0", "ENDSEC".to_owned());
    add("0", "EOF".to_owned());
    groups.iter().map(|(code, value)| format!("{code}\n{value}\n")).collect()
}
