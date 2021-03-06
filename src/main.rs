extern crate ggez;
use ggez::*; use ggez::graphics; use ggez::nalgebra as na;
use ggez::input;

mod body;
use body::Body;

mod physics;
use physics::*;

mod input_type;
use input_type::*;

use rayon::prelude::*;

const G: f32 = 6.674;

#[derive(Clone)]
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
    integrator: Integrator,
    help_menu: bool,
    fast_forward: usize,
    step_size: f32,
    charge: f32,
    input_type: Option<InputVar>,
    input_buffer: String,
}

type Point2 = na::Point2<f32>;
type Vector2 = na::Vector2<f32>;


impl MainState {
    fn new() -> Self {
        let bodies = vec![ //initialize with one massive body in center
            Body::new(
                Point2::new(500.0, 400.0), //position
                300_000.0, //mass
                0.0, //charge
                100.0,  //radius
                Vector2::new(0.0, 0.0)), //velocity
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
            predict_body: Body::new(Point2::new(0.0, 0.0), 1.0, 0.0, 1.0, Vector2::new(0.0, 0.0)),
            predict_speed: 1,
            integrator: Integrator::Verlet,
            help_menu: false,
            fast_forward: 1,
            step_size: 1.0,
            charge: 0.0,
            input_type: None,
            input_buffer: String::new(),
        }
    }
}


impl event::EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let mouse_pos = input::mouse::position(ctx);
        self.mouse_pos.x = (mouse_pos.x - self.offset.x)/self.zoom;
        self.mouse_pos.y = (mouse_pos.y - self.offset.y)/self.zoom;

        if !self.paused{ //physics sim
            (0..self.fast_forward).for_each(|_i|{
                self.bodies = update_velocities_and_collide(&self.bodies, self.integrator, self.step_size);

                (0..self.bodies.len()).for_each(|i|{
                    self.bodies[i].trail_length = self.trail_length;
                })
            })
        }

        //simulate prediction
        if self.mouse_pressed{
            for _i in 0..self.predict_speed { //reimplementation of update_bodies_and_collide() but for only predict body
                self.predict_body.current_accel = self.bodies.iter()
                    .fold(Vector2::new(0.0, 0.0), |acc: Vector2, body|{
                        let r = distance(body.pos, self.predict_body.pos);
                        let a_mag = (G*body.mass)/(r.powi(2));
                        let angle = angle(body.pos, self.predict_body.pos);
                        acc + Vector2::new(a_mag * angle.cos(), a_mag * angle.sin())
                    });

                self.predict_body.trail_length += 1; //infinite trail length
                self.predict_body.update_trail();

                match self.integrator{
                    Integrator::Euler => self.predict_body.update_euler(self.step_size),
                    Integrator::Verlet => self.predict_body.update_verlet(self.step_size),
                };
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        graphics::clear(ctx, graphics::Color::new(0.0, 0.0, 0.0, 0.01));
        if !self.help_menu {
            {
                let input_display = match self.input_type{
                    None => "None",
                    Some(InputVar::Density) => "Density",
                    Some(InputVar::Radius) => "Radius",
                    Some(InputVar::PredictSpeed) => "Predict Speed",
                    Some(InputVar::StepSize) => "Step Size",
                    Some(InputVar::FastForward) => "Sim Speed",
                    Some(InputVar::Charge) => "Charge",
                };

                //top left ui text
                let info = format!(
                    "
                    Offset: {x}, {y}
                    Zoom: {zoom}
                    Density: {density}
                    Charge: {charge}
                    Radius: {radius}
                    Trail length: {trail_length}
                    Prediction Speed: {prediction_speed}
                    Integrator: {method}
                    Sim Speed: {sim_speed}
                    Step Size: {step_size}
                    Inputting: {inputtype} - {inbuffer}
                    Press H for keybinds
                    ",
                    x = self.offset.x,
                    y = self.offset.y, 
                    zoom = self.zoom,
                    density = self.density,
                    charge = self.charge,
                    radius = self.radius,
                    trail_length = self.trail_length,
                    prediction_speed = self.predict_speed,
                    method = format!("{:?}", self.integrator),
                    sim_speed = self.fast_forward,
                    step_size = self.step_size,
                    inputtype = input_display,
                    inbuffer = self.input_buffer.chars().skip(1).collect::<String>());

                let text = graphics::Text::new(info);
                graphics::draw(ctx, &text, graphics::DrawParam::new()).expect("error drawing text");
            }

            let params = graphics::DrawParam::new()
                .dest(self.offset)
                .scale(Vector2::new(self.zoom, self.zoom));
            
            let mut mesh = graphics::MeshBuilder::new();

            for i in 0..self.bodies.len(){ //draw trail and bodies
                if self.trail_length > 1 { //trail
                    let result = mesh.line(
                        &self.bodies[i].trail.as_slices().0,
                        0.25 * self.bodies[i].radius,
                        graphics::Color::new(0.1, 0.25, 1.0, 0.5));

                    match result {
                        Ok(_t) => {},
                        Err(_err) => {},
                    };
                }

                
                let mut r_val = 1.0;
                let mut b_val = 1.0;

                if self.bodies[i].charge < 0.0{
                    r_val += self.bodies[i].charge / 5.0;
                }else {
                    b_val -= self.bodies[i].charge / 5.0;
                }

                let g_val = 1.0 - (r_val - b_val).abs();

                mesh.circle(
                    graphics::DrawMode::fill(),
                    self.bodies[i].pos,
                    self.bodies[i].radius,
                    0.25,
                    graphics::Color::new(r_val, g_val, b_val, 1.0));

            }

            let built_mesh = mesh.build(ctx);

            match built_mesh {
                Ok(mesh) => graphics::draw(ctx, &mesh, params).expect("error drawing body"),
                Err(_err) => {},
            }


            if self.mouse_pressed && self.predict_speed != 0{ // draw prediction
                if self.predict_body.trail.len() > 2{
                    let trail = graphics::Mesh::new_line(
                        ctx,
                        &self.predict_body.trail.as_slices().0,
                        0.25 * self.predict_body.radius,
                        graphics::Color::new(0.0, 1.0, 0.1, 0.4));

                    match trail {
                        Ok(line) => graphics::draw(ctx, &line, params).expect("error drawing trail"),
                        Err(_error) => {},
                    };
                }

                let body = graphics::Mesh::new_circle( //draw prediction body
                    ctx,
                    graphics::DrawMode::fill(),
                    self.predict_body.pos,
                    self.predict_body.radius,
                    0.25,
                    graphics::Color::new(0.0, 1.0, 0.0, 0.8)).expect("error building prediction body");

                graphics::draw(ctx, &body, params).expect("error drawing prediction body");
            }

            if self.mouse_pos != self.start_point && self.mouse_pressed{ //draw preview vector
                let line = graphics::Mesh::new_line(
                    ctx,
                    &[self.start_point, self.mouse_pos][..],
                    0.25 * self.radius,
                    graphics::Color::new(1.0, 1.0, 1.0, 0.8))
                    .expect("error building preview line mesh");

                graphics::draw(ctx, &line, params).expect("error drawing preview line");
            }


            let outline = graphics::Mesh::new_circle( //draw outline
                ctx,
                graphics::DrawMode::fill(),
                if self.mouse_pressed {self.start_point} else {self.mouse_pos},
                self.radius,
                2.0,
                graphics::Color::new(1.0, 1.0, 1.0, 0.25))
                .expect("error building outline");

            graphics::draw(ctx, &outline, params).expect("error drawing outline");
        }else {
            //if help_menu is true
            let help = "
                    Arrow keys to move

                    Scroll to zoom in/out

                    Q/A to increase/decrease radius of next placed body

                    W/S to increase/decrease density (try making it negative)

                    E/D to increase/decrease trail length (removing trails increases performance by a lot)

                    X/Z to increase/decrease prediction speed, setting it to 0 turns of predictions.

                    Left click to place a body, dragging before releasing makes an initial velocity vector.

                    Right click over a body to delete it.

                    G creates a 10x10 grid of bodies with the specified radii and densities.

                    R to reset.

                    Space to pause.

                    I to change integration method

                    1 and 2 to change sim speed (affects performance, not precision)

                    3 and 4 to change step size (affects precision, not performance, lower is better)
                ";

            let text = graphics::Text::new(help);
            graphics::draw(ctx, &text, graphics::DrawParam::new()).expect("error drawing help menu");
        }

        graphics::present(ctx).expect("error rendering");

        if ggez::timer::ticks(ctx) % 60 == 0{
            println!("FPS: {}", ggez::timer::fps(ctx));
            println!("Bodies: {}", self.bodies.len());
        }
        Ok(())
    }

    fn mouse_button_down_event(&mut self, _ctx: &mut Context, button: event::MouseButton, x: f32, y: f32) {
        let zoomed_x = (x - self.offset.x) * (1.0/self.zoom);
        let zoomed_y = (y - self.offset.y) * (1.0/self.zoom);

        match button {
            event::MouseButton::Left => {
                self.start_point = Point2::new(zoomed_x, zoomed_y);
                self.mouse_pressed = true;
            },

            event::MouseButton::Right => {
                println!("Removing body at {} {}", zoomed_x, zoomed_y);
                self.bodies = self.bodies.par_iter() //iterate through meshes and delete any under mouse
                    .filter_map(|body| {
                        let mouse_pointer = Point2::new(zoomed_x, zoomed_y);
                        if distance(mouse_pointer, body.pos) > body.radius {
                            Some(body.clone())
                        }else {
                            None
                        }
                    }).collect();
            }

            _ => {},
        };
    }

    fn mouse_button_up_event(&mut self, _ctx: &mut Context, button: event::MouseButton, x: f32, y: f32) {
        let zoomed_x = (&x - self.offset.x) * (1.0/self.zoom);
        let zoomed_y = (&y - self.offset.y) * (1.0/self.zoom);

        if let event::MouseButton::Left = button{
            self.bodies.push(Body::new(
                    self.start_point,
                    self.radius.powi(3) * self.density,
                    self.charge, 
                    self.radius,
                    Vector2::new((zoomed_x - self.start_point.x)/5.0 * self.zoom, (zoomed_y - self.start_point.y)/5.0 * self.zoom ))
            );
        }

        self.mouse_pressed = false;
    }


    fn mouse_wheel_event(&mut self, _ctx: &mut Context, _x: f32, y: f32) { 
        let prev_zoom = self.zoom;
        self.zoom *= 1.0 + (y * 0.1); 
        let delta_zoom = self.zoom - prev_zoom;
        self.zoom = ((self.zoom * 100_000.0).round())/100_000.0;

        let focus = Vector2::new(self.mouse_pos.x + self.offset.x, self.mouse_pos.y + self.offset.y) * delta_zoom;
        self.offset -= focus;
    }

    fn text_input_event(&mut self, _ctx: &mut Context, character: char){
        match self.input_type{
            None => {},

            _ => {
                if character.is_digit(10) || character == '.'{
                    self.input_buffer.push(character);
                }
            }
        }
    }

    fn key_down_event(&mut self, _ctx: &mut Context, keycode: input::keyboard::KeyCode, _keymods: input::keyboard::KeyMods, _repeat: bool){
        match self.input_type{
            None => { 
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

                self.fast_forward = match keycode {
                    input::keyboard::KeyCode::Key1 => if self.fast_forward == 1 {1} else {self.fast_forward - 1},
                    input::keyboard::KeyCode::Key2 => self.fast_forward + 1,
                    _ => self.fast_forward,
                };

                self.step_size += match keycode {
                    input::keyboard::KeyCode::Key3 => -0.05,
                    input::keyboard::KeyCode::Key4 => 0.05,
                    _ => 0.0,
                };

                self.charge += match keycode {
                    input::keyboard::KeyCode::V => -0.5,
                    input::keyboard::KeyCode::B => 0.5,
                    _ => 0.0,
                };

                match keycode{ //misc keys
                    input::keyboard::KeyCode::Space => self.paused = !self.paused,

                    input::keyboard::KeyCode::G => self.bodies.append(&mut grid(self.offset, self.radius, self.density, self.zoom)),

                    input::keyboard::KeyCode::R => {
                        self.bodies = vec![
                            Body::new(
                                Point2::new(500.0, 400.0),
                                300_000.0,
                                0.0,
                                100.0,
                                Vector2::new(0.0, 0.0)),
                        ];
                        self.zoom = 1.0;
                        self.offset = Point2::new(0.0, 0.0);
                        self.fast_forward = 1;
                    }

                    input::keyboard::KeyCode::I => {
                        self.integrator = match self.integrator {
                            Integrator::Euler => Integrator::Verlet,
                            Integrator::Verlet => Integrator::Euler,
                        };
                    }

                    input::keyboard::KeyCode::H => self.help_menu = !self.help_menu,

                    input::keyboard::KeyCode::Key0 => self.input_type = Some(InputVar::Density),

                    input::keyboard::KeyCode::Key9 => self.input_type = Some(InputVar::Radius),

                    input::keyboard::KeyCode::Key8 => self.input_type = Some(InputVar::PredictSpeed),

                    input::keyboard::KeyCode::Key7 => self.input_type = Some(InputVar::FastForward),

                    input::keyboard::KeyCode::Key6 => self.input_type = Some(InputVar::StepSize),

                    input::keyboard::KeyCode::Key5 => self.input_type = Some(InputVar::Charge),

                    _ => {},
                };
            },

            _ => {
                if keycode == input::keyboard::KeyCode::Return {
                    self.input_buffer = self.input_buffer.chars().skip(1).collect();

                    match self.input_buffer.parse::<f32>(){
                        Err(_e) => {},
                        Ok(num) => {
                            match self.input_type{
                                Some(InputVar::Density) => self.density = num,
                                Some(InputVar::Radius) => self.radius = num,
                                Some(InputVar::PredictSpeed) => self.predict_speed = num as usize,
                                Some(InputVar::FastForward) => self.fast_forward = num as usize,
                                Some(InputVar::StepSize) => self.step_size = num,
                                Some(InputVar::Charge) => self.charge = num,
                                _ => {},
                            }
                        }
                    };

                    self.input_type = None;
                    self.input_buffer = String::new();
                }
            },
        }

        if self.radius < 1.0 {self.radius = 1.0};
        self.radius = (self.radius * 1000.0).round()/1000.0;
        self.density = (self.density * 1000.0).round()/1000.0;
        self.step_size = (self.step_size * 1000.0).round()/1000.0;
    }


    fn mouse_motion_event(&mut self, ctx: &mut Context, _x: f32, _y: f32, dx: f32, dy: f32){

        //this is to make the line when creating a new body and create the preview body
        if self.mouse_pressed {
            self.predict_body = Body::new(
                self.start_point,
                self.radius.powi(3) * self.density,
                0.0,
                self.radius,
                Vector2::new((self.mouse_pos.x - self.start_point.x)/5.0 * self.zoom, (self.mouse_pos.y - self.start_point.y)/5.0 * self.zoom ))
        }

        //move when holding middle click
        if input::mouse::button_pressed(ctx, input::mouse::MouseButton::Middle){
            self.offset.x += dx/self.zoom;
            self.offset.y += dy/self.zoom;
        }
    }
}

pub fn main() -> GameResult{
    let (ctx, event_loop) = &mut ggez::ContextBuilder::new("N-body gravity sim", "Fish")
        .window_setup(ggez::conf::WindowSetup::default().title("N-body gravity sim"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(1000.0, 800.0))
        .build().expect("error building context");
    let state = &mut MainState::new();

    event::run(ctx, event_loop, state)
}

fn grid(start: Point2, radius: f32, density: f32, zoom: f32) -> Vec<Body> {
    //create a 10x10 grid of bodies
    let mut new_bodies: Vec<Body> = Vec::new();

    (1..=10).for_each(|y|{
        (1..=10).for_each(|x| {
            let point = Point2::new((x as f32 * radius * 50.0) - (start.x * (1.0/zoom)), (y as f32 * radius * 50.0) - (start.y * (1.0/zoom)));
            new_bodies.push(Body::new(
                    point,
                    radius.powi(3) * density,
                    0.0,
                    radius,
                    Vector2::new(0.0, 0.0)));
        });
    });

    new_bodies
}
