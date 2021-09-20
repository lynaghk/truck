//! Benchmark Animation
//!
//! In each frame, the NURBS surface is devided into mesh.

use std::sync::{Arc, Mutex};
use std::thread::*;
use truck_modeling::*;
use truck_platform::*;
use truck_rendimpl::*;
use wgpu::*;
mod app;
use app::*;

struct MyApp {
    scene: Scene,
    object: Arc<Mutex<PolygonInstance>>,
    closed: Arc<Mutex<bool>>,
    updated: Arc<Mutex<bool>>,
    thread: Option<JoinHandle<()>>,
}

impl MyApp {
    fn init_surface(degree: usize, division: usize) -> BSplineSurface<Point3> {
        let range = degree + division - 1;
        let knot_vec = KnotVec::uniform_knot(degree, division);
        let mut ctrl_pts = Vec::new();
        for i in 0..=range {
            let u = (i as f64) / (range as f64);
            let mut vec = Vec::new();
            for j in 0..=range {
                let v = (j as f64) / (range as f64);
                vec.push(Point3::new(v, 0.0, u));
            }
            ctrl_pts.push(vec);
        }
        BSplineSurface::new((knot_vec.clone(), knot_vec), ctrl_pts)
    }
    fn init_camera() -> Camera {
        let mut vec0 = Vector4::new(1.5, 0.0, -1.5, 0.0);
        vec0 /= vec0.magnitude();
        let mut vec1 = Vector4::new(-0.5, 1.0, -0.5, 0.0);
        vec1 /= vec1.magnitude();
        let mut vec2 = Vector4::new(1.0, 1.0, 1.0, 0.0);
        vec2 /= vec2.magnitude();
        let vec3 = Vector4::new(1.5, 0.8, 1.5, 1.0);
        let matrix = Matrix4::from_cols(vec0, vec1, vec2, vec3);
        let mut camera = Camera::default();
        camera.matrix = matrix;
        camera
    }
    fn init_thread(
        creator: InstanceCreator,
        object: Arc<Mutex<PolygonInstance>>,
        closed: Arc<Mutex<bool>>,
        updated: Arc<Mutex<bool>>,
        surface: Arc<Mutex<BSplineSurface<Point3>>>,
    ) -> JoinHandle<()> {
        std::thread::spawn(move || {
            let mut time: f64 = 0.0;
            let mut count = 0;
            let mut instant = std::time::Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(1));
                if *closed.lock().unwrap() {
                    break;
                }
                let mut updated = updated.lock().unwrap();
                if *updated {
                    continue;
                }
                *updated = true;
                drop(updated);
                count += 1;
                time += 0.1;
                if count == 100 {
                    let fps_inv = instant.elapsed().as_secs_f64();
                    println!("{}", 100.0 / fps_inv);
                    instant = std::time::Instant::now();
                    count = 0;
                }
                let mut mesh = None;
                match surface.lock() {
                    Ok(mut surface) => {
                        surface.control_point_mut(3, 3)[1] = time.sin();
                        let surface0 = surface.clone();
                        drop(surface);
                        mesh = Some(StructuredMesh::from_surface(
                            &surface0,
                            surface0.parameter_range(),
                            0.01,
                        ));
                    }
                    _ => {}
                }
                let mut another_object = creator.create_instance(
                    &mesh.unwrap(),
                    &PolygonInstanceDescriptor {
                        instance_state: Default::default(),
                        ..Default::default()
                    },
                );
                let mut object = object.lock().unwrap();
                object.swap_vertex(&mut another_object);
            }
        })
    }
}

impl App for MyApp {
    fn init(handler: &DeviceHandler, _info: AdapterInfo) -> MyApp {
        let sample_count = 4;
        let desc = SceneDescriptor {
            camera: MyApp::init_camera(),
            lights: vec![Light {
                position: Point3::new(0.5, 2.0, 0.5),
                color: Vector3::new(1.0, 1.0, 1.0),
                light_type: LightType::Point,
            }],
            sample_count,
            ..Default::default()
        };
        let mut scene = Scene::new(handler.clone(), &desc);
        let creator = scene.instance_creator();
        let surface = Self::init_surface(3, 4);
        let object = creator.create_instance(
            &StructuredMesh::from_surface(&surface, surface.parameter_range(), 0.01),
            &PolygonInstanceDescriptor {
                instance_state: Default::default(),
                ..Default::default()
            },
        );
        scene.add_object(&object);
        let object = Arc::new(Mutex::new(object));
        let closed = Arc::new(Mutex::new(false));
        let updated = Arc::new(Mutex::new(false));
        let thread = Some(MyApp::init_thread(
            creator,
            Arc::clone(&object),
            Arc::clone(&closed),
            Arc::clone(&updated),
            Arc::new(Mutex::new(surface)),
        ));
        MyApp {
            scene,
            object,
            closed,
            updated,
            thread,
        }
    }

    fn app_title<'a>() -> Option<&'a str> {
        Some("BSpline Benchmark Animation")
    }

    fn update(&mut self, _: &DeviceHandler) {
        let mut updated = self.updated.lock().unwrap();
        if *updated {
            let object = self.object.lock().unwrap();
            self.scene.update_vertex_buffer(&*object);
            *updated = false;
        }
    }

    fn render(&mut self, view: &TextureView) {
        self.scene.render_scene(view);
    }
    fn closed_requested(&mut self) -> winit::event_loop::ControlFlow {
        *self.closed.lock().unwrap() = true;
        self.thread.take().unwrap().join().unwrap();
        winit::event_loop::ControlFlow::Exit
    }
}

fn main() {
    MyApp::run();
}
