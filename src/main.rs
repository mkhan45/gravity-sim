#[macro_use] extern crate microprofile;
extern crate ggez;
use ggez::*; use ggez::graphics; use ggez::nalgebra as na;
use ggez::input;

mod body;
use body::Body;

mod physics;
use physics::*;

use rayon::prelude::*;
use std::thread;

const G: f32 = 6.674;

struct MainState {
    bodies: Vec<Body>,
    start_point: Point2,
    zoom: f32,
    offset: Point2,
    density: f32,
    radius: f32,
    mouse_pos: Point2,
    trail_length: usize,
    mouse_pressed: bool,
    paused: bool,
    predict_body: Body,
    predict_speed: usize,
}

type Point2 = na::Point2<f32>;
type Vector2 = na::Vector2<f32>;


impl MainState {
    fn new(ctx: &mut Context) -> Self {
        let width = ctx.conf.window_mode.width as f32;
        let height = ctx.conf.window_mode.height as f32;
        let bodies = vec![
            Body::new(
                Point2::new(width/2.0, height/2.0),
                300000.0,
                100.0,
                Vector2::new(0.0, 0.0)),
        ];

        MainState {
            bodies,
            start_point: Point2::new(0.0, 0.0),
            zoom: 1.0,
            offset: Point2::new(0.0, 0.0),
            density: 0.05,
            radius: 10.0,
            mouse_pos: Point2::new(0.0, 0.0),
            trail_length: 30,
            mouse_pressed: false,
            paused: false,
            predict_body: Body::new(Point2::new(0.0, 0.0), 1.0, 1.0, Vector2::new(0.0, 0.0)),
            predict_speed: 1,
        }
    }
}


impl event::EventHandler for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        microprofile::flip();
        if !self.paused{
            self.bodies = update_velocities_and_collide(&self.bodies);
            for i in 0..self.bodies.len(){
                self.bodies[i].update();
                self.bodies[i].trail_length = self.trail_length;
            }
        }
        
        for i in (0..self.predict_speed){
            if self.mouse_pressed{
                self.predict_body.past_accel = self.bodies.iter()
                    .fold(Vector2::new(0.0, 0.0), |acc: Vector2, body|{
                        let r = distance(&body.pos, &self.predict_body.pos);
                        let a_mag = (G*body.mass)/(r.powf(2.0));
                        let angle = angle(&body.pos, &self.predict_body.pos);
                        acc + Vector2::new(a_mag * angle.cos(), a_mag * angle.sin())
                    });
                self.predict_body.trail_length = self.predict_body.trail.len() + 2;
                self.predict_body.update();
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        microprofile::scope!("Draw", "Main");
        graphics::clear(ctx, graphics::Color::new(0.0, 0.0, 0.0, 1.0));

        let info = format!(
            "
            Offset: {x}, {y}
            Zoom: {zoom}
            Density: {density}
            Radius: {radius}
            Trail length: {trail_length}
            Prediction Speed: {prediction_speed}
            ",
            x = self.offset.x, y = self.offset.y, zoom = self.zoom, density = self.density, radius = self.radius, trail_length = self.trail_length, prediction_speed = self.predict_speed);

        let text = graphics::Text::new(info);
        graphics::draw(ctx, &text, graphics::DrawParam::new()).expect("error drawing text");

        let mut params = graphics::DrawParam::new();

        params = params.dest(self.offset);
        params = params.scale(Vector2::new(self.zoom, self.zoom));


        for i in 0..self.bodies.len(){ //draw trail and bodies
            if self.trail_length > 1 { //trail
                let trail = graphics::Mesh::new_line(
                    ctx,
                    &self.bodies[i].trail.as_slices().0,
                    0.25 * self.bodies[i].radius,
                    graphics::Color::new(0.1, 0.25, 1.0, 0.5)
                    );

                match trail {
                    Ok(line) => graphics::draw(ctx, &line, params).expect("error drawing trail"),
                    Err(_error) => {},
                };
            }

            let body = graphics::Mesh::new_circle( //draw body
                ctx,
                graphics::DrawMode::fill(),
                self.bodies[i].pos,
                self.bodies[i].radius,
                2.0,
                graphics::Color::new(1.0, 1.0, 1.0, 1.0),
                )?;

            graphics::draw(ctx, &body, params).expect("error drawing body");
        }

        if self.mouse_pressed && self.predict_speed != 0{ //prediction
            if self.predict_body.trail.len() > 2{
                let trail = graphics::Mesh::new_line(
                        ctx,
                        &self.predict_body.trail.as_slices().0,
                        0.25 * self.predict_body.radius,
                        graphics::Color::new(0.0, 1.0, 0.1, 0.4)
                        );

                    match trail {
                        Ok(line) => graphics::draw(ctx, &line, params).expect("error drawing trail"),
                        Err(_error) => {},
                    };
            }

            let body = graphics::Mesh::new_circle( //draw body
                ctx,
                graphics::DrawMode::fill(),
                self.predict_body.pos,
                self.predict_body.radius,
                2.0,
                graphics::Color::new(0.0, 1.0, 0.0, 0.8),
                )?;

            graphics::draw(ctx, &body, params).expect("error drawing prediction body");

        }

        if self.mouse_pos != self.start_point && self.mouse_pressed{ //draw vector
            let line = graphics::Mesh::new_line(
                ctx,
                &vec![self.start_point, self.mouse_pos][..],
                0.25 * self.radius,
                graphics::Color::new(1.0, 1.0, 1.0, 0.8),
                )?;

            graphics::draw(ctx, &line, params).expect("error drawing preview line");
        }


        let outline = graphics::Mesh::new_circle( //draw outline
            ctx,
            graphics::DrawMode::fill(),
            if self.mouse_pressed {self.start_point} else {self.mouse_pos},
            self.radius,
            2.0,
            graphics::Color::new(1.0, 1.0, 1.0, 0.25),
            )?;

        graphics::draw(ctx, &outline, params).expect("error drawing outline");
        

        graphics::present(ctx).expect("error rendering");


        if ggez::timer::ticks(ctx) % 60 == 0{
            println!("FPS: {}", ggez::timer::fps(ctx));
            println!("Bodies: {}", self.bodies.len());
        }
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: event::MouseButton, x: f32, y: f32) {
        let zoomed_x = (&x - self.offset.x) * (1.0/self.zoom);
        let zoomed_y = (&y - self.offset.y) * (1.0/self.zoom);

        match button {
            event::MouseButton::Left => {
                self.start_point = Point2::new(zoomed_x, zoomed_y);
                self.mouse_pressed = true;
            },

            event::MouseButton::Right => {
                println!("Removing body at {} {}", zoomed_x, zoomed_y);
                self.bodies = self.bodies.par_iter()
                    .filter_map(|body| {
                        let mouse_pointer = Point2::new(zoomed_x, zoomed_y);
                        if distance(&mouse_pointer, &body.pos) > body.radius {
                            Some(body.clone())
                        }else {
                            None
                        }
                    })
                .collect();
            }

            _ => {},
        };
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: event::MouseButton, x: f32, y: f32) {
        let zoomed_x = (&x - self.offset.x) * (1.0/self.zoom);
        let zoomed_y = (&y - self.offset.y) * (1.0/self.zoom);

        match button {
            event::MouseButton::Left => {
                self.bodies.push(Body::new(
                        self.start_point,
                        self.radius.powf(3.0) * self.density,
                        self.radius,
                        Vector2::new((zoomed_x - self.start_point.x)/5.0 * self.zoom, (zoomed_y - self.start_point.y)/5.0 * self.zoom ),
                        ));
            },

            _ => {},
        }

        self.mouse_pressed = false;
    }


    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) { self.zoom *= 1.0 + (y as f32 * 0.1); 
    }

    fn key_down_event(&mut self, _ctx: &mut Context, keycode: input::keyboard::KeyCode, _keymods: input::keyboard::KeyMods, _repeat: bool){
        self.offset.y += match keycode{
            input::keyboard::KeyCode::Up => 50.0,
            input::keyboard::KeyCode::Down => -50.0,
            _ => 0.0,
        };

        self.offset.x += match keycode{
            input::keyboard::KeyCode::Left => 50.0,
            input::keyboard::KeyCode::Right => -50.0,
            _ => 0.0,
        };

        self.density += match keycode{
            input::keyboard::KeyCode::W => 0.05,
            input::keyboard::KeyCode::S => -0.05,
            _ => 0.0,
        };

        self.radius += match keycode{
            input::keyboard::KeyCode::Q => 1.0,
            input::keyboard::KeyCode::A => -1.0,
            _ => 0.0,
        };

        self.trail_length = match keycode{
            input::keyboard::KeyCode::E => self.trail_length + 1,
            input::keyboard::KeyCode::D => if self.trail_length != 0 {self.trail_length - 1} else {0},
            _ => self.trail_length,
        };

        self.predict_speed = match keycode {
            input::keyboard::KeyCode::X => self.predict_speed + 1,
            input::keyboard::KeyCode::Z => if self.predict_speed != 0 {self.predict_speed - 1} else {0},
            _ => self.predict_speed,
        };

        match keycode{
            input::keyboard::KeyCode::Space => {
                self.paused = !self.paused;
            },

            input::keyboard::KeyCode::G => {
                self.bodies.append(&mut grid(&self.offset, &self.radius, &self.density, &self.zoom));
            }

            input::keyboard::KeyCode::R => self.bodies = Vec::new(),

            _ => {},
        };

        if self.radius < 1.0 {self.radius = 1.0};
    }

    fn mouse_motion_event(&mut self, _ctx: &mut Context, _x: f32, _y: f32, _dx: f32, _dy: f32){
        let zoomed_x = (&_x - self.offset.x) * (1.0/self.zoom);
        let zoomed_y = (&_y - self.offset.y) * (1.0/self.zoom);
        self.mouse_pos = Point2::new(zoomed_x, zoomed_y);
        if self.mouse_pressed {
            self.predict_body = Body::new(
                self.start_point,
                self.radius.powf(3.0) * self.density,
                self.radius,
                Vector2::new((zoomed_x - self.start_point.x)/5.0 * self.zoom, (zoomed_y - self.start_point.y)/5.0 * self.zoom ),
                )
        }
    }
}

pub fn main() -> GameResult{
    let (ctx, event_loop) = &mut ggez::ContextBuilder::new("N-body gravity sim", "Fish")
        .window_setup(ggez::conf::WindowSetup::default().title("N-body gravity sim"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(1000.0, 800.0))
        .build()?;
    let state = &mut MainState::new(ctx);

    microprofile::init();
    microprofile::set_enable_all_groups(true);
    event::run(ctx, event_loop, state)
}

fn grid(start: &Point2, radius: &f32, density: &f32, zoom: &f32) -> Vec<Body> {
    let mut new_bodies: Vec<Body> = Vec::new();

    (1..=10).for_each(|y|{
        (1..=10).for_each(|x| {
            let point = Point2::new((x as f32 * radius * 50.0) - (start.x * (1.0/zoom)), (y as f32 * radius * 50.0) - (start.y * (1.0/zoom)));
            new_bodies.push(Body::new(
                    point,
                    radius.powf(3.0) * density,
                    *radius,
                    Vector2::new(0.0, 0.0)));
        });
    });

    new_bodies
}
