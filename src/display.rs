// SPDX-License-Identifier: EUPL-1.2

use anyhow::Result;
use lockfree::channel::spsc;
use lockfree::channel::RecvErr;
use macroquad::prelude::*;
use miniquad;

use crate::audio_analyzer;
use crate::screensaver;

const R: f32 = 0.000976;
const D_MIN: f32 = 0.146;
const D_MAX: f32 = 1.0;
const S_R: f32 = 0.1;
const S_V: f32 = 0.382;

const BPM_MIN: f32 = 200.0;

pub fn run(mut event_rx: spsc::Receiver<audio_analyzer::Event>) -> () {
    macroquad::Window::from_config(
        Conf {
            window_title: "isis".to_owned(),
            fullscreen: true,
            ..Default::default()
        },
        async move {
            if let Err(err) = arun(&mut event_rx).await {
                {
                    let lvl = miniquad::log::Level::Error;
                    miniquad::log::__private_api_log_lit(
                        &format!("Error: {0:?}", err),
                        lvl,
                        &("isis", "isis", "src/display.rs", 8u32),
                    );
                };
            }
        },
    );
}

pub async fn arun(event_rx: &mut spsc::Receiver<audio_analyzer::Event>) -> Result<()> {
    let texture: Texture2D = load_texture("chess.png").await.unwrap();

    let lens_material = load_material(
        ShaderSource::Glsl {
            vertex: LENS_VERTEX_SHADER,
            fragment: LENS_FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("Center", UniformType::Float2),
                UniformDesc::new("sign_o", UniformType::Float1),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    let mut audio_bpm: f32 = BPM_MIN;
    let mut audio_rms: f32 = 0.0;

    let mut bpm: f32 = 0.0;
    let mut rms: f32 = 0.0;

    let mut frame_time = get_frame_time();
    let mut theta: f32 = 0.0;
    let mut sign_a: f32 = 1.0;
    // TODO Find interesting usage for this one
    //let mut sign_o: f32 = -sign_a;

    show_mouse(false);

    loop {
        // receive events
        loop {
            match event_rx.recv() {
                Ok(audio_analyzer::Event::Reset) => {
                    audio_bpm = BPM_MIN;
                    audio_rms = 0.0;

                    sign_a = -sign_a;
                }
                Ok(audio_analyzer::Event::Tempo {
                    average: bpm,
                    accuracy: _,
                }) => {
                    screensaver::reset()?;
                    audio_bpm = bpm;
                }
                Ok(audio_analyzer::Event::Volume { average: rms }) => {
                    audio_rms = rms;
                }
                Err(RecvErr::NoMessage) => {
                    break;
                }
                Err(err) => {
                    eprintln!("{:?}", err);
                }
            }
        }

        // compute state
        let bpm_delta = audio_bpm - bpm;
        if bpm_delta != 0.0 {
            bpm += bpm_delta * frame_time * S_R;
        }

        let rms_delta = audio_rms - rms;
        if rms_delta != 0.0 {
            rms += rms_delta * frame_time * S_V;
        }

        // animate
        let screen_size = vec2(screen_width(), screen_height());
        let screen_center = screen_size / 2.0;
        let screen_center_min = screen_center.x.min(screen_center.y);

        theta += frame_time * bpm * R * sign_a;

        if theta > std::f32::consts::PI * 2.0 {
            theta = 0.0
        };
        if theta < 0.0 {
            theta = std::f32::consts::PI * 2.0
        };

        let lens_distance = screen_center_min * D_MIN.lerp(D_MAX, rms);
        let mut lens_center = screen_center + (lens_distance * std::f32::consts::PI.cos());
        lens_center = vec2(
            theta.cos() * (lens_center.x - screen_center.x)
                - theta.sin() * (lens_center.y - screen_center.y)
                + screen_center.x,
            theta.sin() * (lens_center.x - screen_center.x)
                + theta.cos() * (lens_center.y - screen_center.y)
                + screen_center.y,
        );

        // draw
        clear_background(WHITE);
        draw_texture_ex(
            &texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(screen_size),
                ..Default::default()
            },
        );

        lens_material.set_uniform("Center", lens_center);
        //lens_material.set_uniform("sign_o", sign_o);

        gl_use_material(&lens_material);
        draw_circle(lens_center.x, lens_center.y, screen_center_min * 5.0, RED);
        gl_use_default_material();

        // wait for next frame
        let minimum_frame_time = 1. / 24.; // 24 FPS
        frame_time = get_frame_time();
        if frame_time < minimum_frame_time {
            let time_to_sleep = minimum_frame_time - frame_time;
            std::thread::sleep(std::time::Duration::from_millis(
                time_to_sleep as u64 * 1000,
            ));
            frame_time += time_to_sleep;
        }

        next_frame().await
    }
}

const LENS_FRAGMENT_SHADER: &'static str = r#"#version 100
precision lowp float;

varying vec2 uv;
varying vec2 uv_screen;
varying vec2 center;
uniform float sign_o;

uniform sampler2D _ScreenTexture;

void main() {
    float gradient = length(uv);
    vec2 uv_zoom = (uv_screen - center) * gradient + center;

    gl_FragColor = texture2D(_ScreenTexture, uv_zoom);

    float lum = 1.0; // 0.382;
    vec4 a = vec4(0.4951, 0.2822, 1.0, 1.0);
    vec4 o = vec4(0.706, 0.329, 1.0, 1.0);

    if (gl_FragColor == vec4(1.0)) {
        gl_FragColor = a;
    } else {
        //gl_FragColor = vec4(0.831, 0.235, 1.0, 1.0);
        gl_FragColor = o;
    }

    gl_FragColor = gl_FragColor * lum;
}
"#;

const LENS_VERTEX_SHADER: &'static str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;

varying lowp vec2 center;
varying lowp vec2 uv;
varying lowp vec2 uv_screen;

uniform mat4 Model;
uniform mat4 Projection;

uniform vec2 Center;

void main() {
    vec4 res = Projection * Model * vec4(position, 1);
    vec4 c = Projection * Model * vec4(Center, 0, 1);

    uv_screen = res.xy / 2.0 + vec2(0.5, 0.5);
    center = c.xy / 2.0 + vec2(0.5, 0.5);
    uv = texcoord;

    gl_Position = res;
}
";
