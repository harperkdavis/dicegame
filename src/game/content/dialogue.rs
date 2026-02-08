use std::f64;

use raylib::prelude::*;

use crate::res::Res;

#[derive(Clone, Copy, Debug)]
pub enum Style {
    Emphasized,
    Group,
}

#[derive(Debug)]
pub struct Span {
    pub text: String,
    pub style: Option<Style>,
}

fn parse_spans(str: &str) -> eyre::Result<Vec<Span>> {
    let mut start = 0;
    let mut end = 0;

    let mut spans = Vec::new();
    let mut current_style = None;

    while end < str.len() {
        if &str[end..=end] == "\\" {
            spans.push(Span {
                text: str[start..end].to_string(),
                style: current_style,
            });
            match current_style {
                Some(_) => {
                    if end - start > 0 {
                        start = end + 1;
                        current_style = None;
                    }
                }
                None => {
                    end += 1;
                    start = end + 1;
                    current_style = Some(match &str[end..=end] {
                        "!" => Style::Emphasized,
                        "&" => Style::Group,
                        _ => {
                            return Err(eyre::eyre!(
                                "invalid style identifier: \\{}",
                                &str[end..=end]
                            ));
                        }
                    });
                }
            }
        }

        end += 1;
    }

    spans.push(Span {
        text: str[start..end].to_string(),
        style: current_style,
    });

    println!("{spans:?}");
    Ok(spans)
}

fn find_breaks(full_text: &str, max_width: usize) -> Vec<usize> {
    let words = full_text.split_whitespace().collect::<Vec<_>>();

    let mut current = 0;
    let mut line = 0;

    let mut breaks = vec![0];
    for word in words {
        let next = line + word.len();
        if next > max_width {
            breaks.push(current);
            line = word.len();
        } else {
            line = next + 1;
        }
        current += word.len() + 1;
    }

    println!("{breaks:?}");

    breaks
}

fn get_line(breaks: &[usize], index: usize) -> usize {
    breaks
        .iter()
        .enumerate()
        .rfind(|(_, break_index)| index >= **break_index)
        .map(|(i, _)| i)
        .unwrap_or(0)
}

#[derive(Default)]
pub struct Meta {
    speaker: Option<String>,
}

impl TryFrom<&str> for Meta {
    type Error = eyre::Report;
    fn try_from(_: &str) -> eyre::Result<Self> {
        Ok(Self::default())
    }
}

pub struct Line {
    pub meta: Meta,
    pub full_text: String,
    pub spans: Vec<Span>,
    pub breaks: Vec<usize>,
}

struct Character {
    index: usize,

    index_in_span: usize,
    span_length: usize,

    character: char,
    line: usize,
    style: Option<Style>,
}

const READ_SPEED_CPS: f64 = 30.0;
impl Character {
    fn delay_time(&self) -> f64 {
        if let Some(Style::Group) = self.style {
            return if self.index_in_span == self.span_length - 1 {
                self.span_length as f64 * 1.0 / READ_SPEED_CPS
            } else {
                0.0
            };
        };
        match self.character {
            '.' => 4.0 / READ_SPEED_CPS,
            ',' => 2.0 / READ_SPEED_CPS,
            _ => 1.0 / READ_SPEED_CPS,
        }
    }
}

impl Line {
    pub fn len(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }

    fn characters(&self) -> impl Iterator<Item = Character> {
        self.spans
            .iter()
            .flat_map(|span| {
                span.text
                    .chars()
                    .enumerate()
                    .map(|(iis, c)| (c, span.style, iis, span.text.len()))
            })
            .enumerate()
            .map(|(index, (character, style, iis, sl))| Character {
                index,
                index_in_span: iis,
                span_length: sl,
                character,
                line: get_line(&self.breaks, index),
                style,
            })
    }

    pub fn draw(
        &self,
        d: &mut RaylibMode2D<RaylibDrawHandle>,
        res: &Res,
        font: &Font,
        dialogue_elapsed: f64,
        line_elapsed: Option<f64>,
    ) -> bool {
        let anim_dialogue = 0.5_f64.powf(dialogue_elapsed * 8.0) * 160.0;
        let anim_line = line_elapsed.map_or(0.0, |elapsed| 0.5_f64.powf(elapsed * 8.0)) * -4.0;
        let anim_y = (anim_dialogue + anim_line) as i32;

        d.draw_rectangle(79, 349 + anim_y, 442, 122, Color::WHITE);
        d.draw_rectangle(80, 350 + anim_y, 440, 120, Color::BLACK);

        let mut reveal = 0.0;
        let mut drew_all = true;

        for c in self.characters() {
            if let Some(elapsed) = line_elapsed {
                if reveal > elapsed {
                    if reveal < elapsed + d.get_frame_time() as f64 && !c.character.is_whitespace()
                    {
                        res.snd("dialogue").play();
                    }
                    drew_all = false;
                    break;
                }
            }
            reveal += c.delay_time();

            d.draw_text_ex(
                font,
                &c.character.to_string(),
                Vector2::new(
                    98.0 + (c.index - self.breaks[c.line]) as f32 * 16.0,
                    364.0 + c.line as f32 * 24.0 + anim_y as f32,
                ),
                16.0,
                1.0,
                if matches!(c.style, Some(Style::Emphasized)) {
                    Color::YELLOW
                } else {
                    Color::WHITE
                },
            );
        }

        drew_all
    }
}

impl TryFrom<&str> for Line {
    type Error = eyre::Report;
    fn try_from(value: &str) -> eyre::Result<Self> {
        // Standardize whitespace
        let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
        let (meta, spans) = if value.starts_with("[") {
            let (meta, content) = value
                .split_once(']')
                .ok_or_else(|| eyre::eyre!("failed to parse metadata: unclosed [: {value:?}"))?;
            let meta = &meta[1..];
            (Meta::try_from(meta.trim())?, parse_spans(content.trim())?)
        } else {
            (Meta::default(), parse_spans(&value)?)
        };

        let full_text = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        let breaks = find_breaks(&full_text, 25);
        Ok(Line {
            meta,
            full_text: value,
            spans,
            breaks,
        })
    }
}
