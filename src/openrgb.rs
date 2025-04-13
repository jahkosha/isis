// SPDX-License-Identifier: EUPL-1.2

// TODO Integrate and test
/*
use std::error::Error;

use openrgb::data::ColorMode;
use openrgb::data::Mode;
use openrgb::data::ModeFlag::*;
use rgb::RGB;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[test]
    fn it_works() {
        tokio_test::block_on(async {
            let mut session = Session::new("Philips Wiz".to_string()).await.unwrap();
            session
                .update_all(0.3, RGB::new(u8::MIN, u8::MIN, u8::MAX))
                .await
                .unwrap()
        });
    }
}

struct Session {
    client: openrgb::OpenRGB<tokio::net::TcpStream>,
    ctrls: Vec<u32>,
}

impl Session {
    async fn new(ctrl_name: String) -> Result<Session, Box<dyn Error>> {
        let client = openrgb::OpenRGB::connect().await?;

        let mut ctrls = vec![];
        for ctrl_id in 0..client.get_controller_count().await? {
            let ctrl = client.get_controller(ctrl_id).await?;
            if ctrl.name == ctrl_name {
                println!("controller {}: {:#?}", ctrl_id, ctrl.location);
                ctrls.push(ctrl_id);
                client.set_custom_mode(ctrl_id).await?;
            }
        }

        Ok(Session { client, ctrls })
    }

    async fn update_all(&mut self, brightness: f32, color: RGB<u8>) -> Result<(), Box<dyn Error>> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;

        for &ctrl_id in &self.ctrls {
            let mode = Mode {
                name: "Direct".to_string(),
                value: 0,
                flags: HasBrightness | HasPerLEDColor,
                speed_min: None,
                speed_max: None,
                brightness_min: Some(10),
                brightness_max: Some(100),
                brightness: Some(10 + (brightness * 90.0) as u32),
                colors_min: None,
                colors_max: None,
                speed: None,
                direction: None,
                color_mode: Some(ColorMode::PerLED),
                colors: vec![],
            };

            self.client.update_mode(ctrl_id, 0, mode).await?;
            self.client.update_led(ctrl_id, 0, color).await?;
        }

        self.ctrls.shuffle(&mut thread_rng());

        Ok(())
    }
}
*/
