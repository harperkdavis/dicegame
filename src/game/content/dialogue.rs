use std::{f64, str::FromStr};

use raylib::prelude::*;
use serde::Deserialize;

use crate::{Str, res::Res};

#[derive(Clone, Copy, Debug)]
pub enum Style {
    Emphasized,
    Group,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub style: Option<Style>,
}

fn parse_spans(str: &str, meta: &Meta) -> eyre::Result<Vec<Span>> {
    if meta.group_every.is_some_and(|b| b) {
        return Ok(str
            .split_whitespace()
            .map(|s| Span {
                text: format!("{s} "),
                style: Some(Style::Group),
            })
            .collect());
    }
    let mut start = 0;
    let mut end = 0;

    let mut spans = Vec::new();
    let mut current_style = None;

    while end < str.len() {
        if &str[end..=end] == "/" {
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
                                "invalid style identifier: /{}",
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

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Meta {
    speaker: Option<String>,
    face: Option<String>,
    group_every: Option<bool>,
}

impl TryFrom<&str> for Meta {
    type Error = eyre::Report;
    fn try_from(value: &str) -> eyre::Result<Self> {
        toml_edit::de::from_str(value).map_err(|e| eyre::eyre!("failed to deserialize: {e}"))
    }
}

#[derive(Clone, Debug)]
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

pub fn draw_dialogue_box(
    d: &mut impl RaylibDraw,
    dialogue_elapsed: f64,
    line_elapsed: Option<f64>,
) -> i32 {
    let anim_dialogue = 0.5_f64.powf(dialogue_elapsed * 32.0) * 160.0;
    let anim_line = line_elapsed.map_or(0.0, |elapsed| 0.5_f64.powf(elapsed * 16.0)) * -4.0;
    let anim_y = (anim_dialogue + anim_line) as i32;

    d.draw_rectangle(79, 349 + anim_y, 442, 122, Color::WHITE);
    d.draw_rectangle(80, 350 + anim_y, 440, 120, Color::BLACK);

    anim_y
}

pub fn draw_choice_box(
    d: &mut impl RaylibDraw,
    font: &Font,
    choice_elapsed: f64,
    select_elapsed: Option<f64>,
    choices: &[String],
    select: usize,
) {
    let anim_y = draw_dialogue_box(d, choice_elapsed, select_elapsed);

    let start_y = 350 + 40 + anim_y - (choices.len().saturating_sub(1) as i32 / 2) * 20;
    let start_x = if choices.len() > 1 {
        90_i32
    } else {
        80 + 440 / 2 - 200 / 2
    };

    for (i, c) in choices.iter().enumerate() {
        let x = start_x + (i as i32 % 2) * 220;
        let y = start_y + (i as i32 / 2) * 40;
        if select == i {
            d.draw_rectangle(
                x + 1,
                y + 1,
                198,
                38,
                Color::WHITE.alpha(f32::sin(choice_elapsed as f32 * 8.0) * 0.25 + 0.75),
            );
        }
        d.draw_rectangle(x + 2, y + 2, 196, 36, Color::BLACK);
        d.draw_text_ex(
            font,
            c.as_str(),
            Vector2::new((x + 12) as f32, (y + 12) as f32),
            16.0,
            1.0,
            if select == i {
                Color::WHITE
            } else {
                Color::GRAY
            },
        );
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
        d: &mut impl RaylibDraw,
        res: &Res,
        font: &Font,
        dialogue_elapsed: f64,
        line_elapsed: Option<f64>,
        delta: f32,
    ) -> bool {
        let anim_y = draw_dialogue_box(d, dialogue_elapsed, line_elapsed);

        if let Some(speaker) = self.meta.speaker.as_deref() {
            /*
            d.draw_rectangle(79, 309 + anim_y, 122, 36, Color::WHITE);
            d.draw_rectangle(80, 310 + anim_y, 120, 34, Color::BLACK);

            d.draw_text_ex(
                font,
                speaker,
                Vector2::new(95.0, (321 + anim_y) as f32),
                16.0,
                1.0,
                Color::WHITE,
            );
            */

            let face = self.meta.face.as_deref().unwrap_or("neutral");
            d.draw_texture(
                res.tex(
                    &Str::from_str(format!("face/{}/{face}", speaker.to_lowercase()).as_str())
                        .unwrap(),
                ),
                80,
                342 + anim_y,
                Color::WHITE,
            );
        }

        let mut reveal = 0.0;
        let mut drew_all = true;

        for c in self.characters() {
            if let Some(elapsed) = line_elapsed
                && reveal > elapsed
            {
                if reveal < elapsed + delta as f64 && !c.character.is_whitespace() {
                    res.snd("dialogue").stop();
                    res.snd("dialogue").play();
                }
                drew_all = false;
                break;
            }
            reveal += c.delay_time();

            d.draw_text_ex(
                font,
                &c.character.to_string(),
                Vector2::new(
                    98.0 + (c.index - self.breaks[c.line]) as f32 * 16.0
                        + if self.meta.speaker.is_some() {
                            120.0
                        } else {
                            0.0
                        },
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
        let (meta, spans) = if value.starts_with("{") {
            let (meta, content) = value
                .rsplit_once('}')
                .ok_or_else(|| eyre::eyre!("failed to parse metadata: unclosed {{: {value:?}"))?;
            // cheeky hack
            let meta = &meta[1..].replace(",", "\n");
            let meta = Meta::try_from(meta.trim())?;
            let spans = parse_spans(content.trim(), &meta)?;
            (meta, spans)
        } else {
            (Meta::default(), parse_spans(&value, &Meta::default())?)
        };

        let full_text = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        let max_width = if meta.speaker.is_some() { 18 } else { 25 };
        let breaks = find_breaks(&full_text, max_width);

        Ok(Line {
            meta,
            full_text: value,
            spans,
            breaks,
        })
    }
}

pub fn parse_lines(s: &str) -> eyre::Result<Vec<Line>> {
    let mut lines = Vec::new();

    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        lines.push(Line::try_from(line)?);
    }

    Ok(lines)
}
