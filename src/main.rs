use glfw::{Action, MouseButton, Context as _, Key, WindowEvent};
use luminance::context::GraphicsContext as _;
use luminance::pipeline::PipelineState;
use luminance::render_state::RenderState;
use luminance::tess::Mode;
use luminance::{Semantics, Vertex};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};

// We get the shader at compile time from local files
const VS: &'static str = include_str!("simple-vs.glsl");
const FS: &'static str = include_str!("simple-fs.glsl");

// Vertex semantics. Those are needed to instruct the GPU how to select vertex’s attributes from
// the memory we fill at render time, in shaders. You don’t have to worry about them; just keep in
// mind they’re mandatory and act as “protocol” between GPU’s memory regions and shaders.
//
// We derive Semantics automatically and provide the mapping as field attributes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum Semantics {
  // - Reference vertex positions with the "co" variable in vertex shaders.
  // - The underlying representation is [f32; 2], which is a vec2 in GLSL.
  // - The wrapper type you can use to handle such a semantics is VertexPosition.
  #[sem(name = "co", repr = "[f32; 2]", wrapper = "VertexPosition")]
  Position,
  // - Reference vertex colors with the "color" variable in vertex shaders.
  // - The underlying representation is [u8; 3], which is a uvec3 in GLSL.
  // - The wrapper type you can use to handle such a semantics is VertexColor.
  #[sem(name = "color", repr = "[u8; 3]", wrapper = "VertexColor")]
  Color,
}

// Our vertex type.
//
// We derive the Vertex trait automatically and we associate to each field the semantics that must
// be used on the GPU. The proc-macro derive Vertex will make sur for us every field we use have a
// mapping to the type you specified as semantics.
//
// Currently, we need to use #[repr(C))] to ensure Rust is not going to move struct’s fields around.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "Semantics")]
struct Vertex {
  pos: VertexPosition,
  // Here, we can use the special normalized = <bool> construct to state whether we want integral
  // vertex attributes to be available as normalized floats in the shaders, when fetching them from
  // the vertex buffers. If you set it to "false" or ignore it, you will get non-normalized integer
  // values (i.e. value ranging from 0 to 255 for u8, for instance).
  #[vertex(normalized = "true")]
  rgb: VertexColor,
}

// The vertices. We define two triangles.
const TRI_VERTICES: [Vertex; 6] = [
  // First triangle – an RGB one.
  Vertex::new(
    VertexPosition::new([0.5, -0.5]),
    VertexColor::new([0, 255, 0]),
  ),
  Vertex::new(
    VertexPosition::new([0.0, 0.5]),
    VertexColor::new([0, 0, 255]),
  ),
  Vertex::new(
    VertexPosition::new([-0.5, -0.5]),
    VertexColor::new([255, 0, 0]),
  ),
  // Second triangle, a purple one, positioned differently.
  Vertex::new(
    VertexPosition::new([-0.5, 0.5]),
    VertexColor::new([255, 51, 255]),
  ),
  Vertex::new(
    VertexPosition::new([0.0, -0.5]),
    VertexColor::new([51, 255, 255]),
  ),
  Vertex::new(
    VertexPosition::new([0.5, 0.5]),
    VertexColor::new([51, 51, 255]),
  ),
];

// Indices into TRI_VERTICES to use to build up the triangles.
const TRI_INDICES: [u8; 6] = [
  0, 1, 2, // First triangle.
  3, 4, 5, // Second triangle.
];

fn main() {
  // First thing first: we create a new surface to render to and get events from.
  let dim = WindowDim::Windowed {
    width: 960,
    height: 540,
  };
  let mut surface = GlfwSurface::new_gl33(
    "Hello, world; from OpenGL 3.3!",
    WindowOpt::default().set_dim(dim),
  )
  .expect("GLFW surface creation");

  // We need a program to “shade” our triangles and to tell luminance which is the input vertex
  // type, and we’re not interested in the other two type variables for this sample.

  let mut program = surface
    .new_shader_program::<Semantics, (), ()>()
    .from_strings(VS, None, None, FS)
    .expect("program creation")
    .ignore_warnings();

  // Create indexed tessellation; that is, the vertices will be picked by using the indexes provided
  // by the second slice and this indexes will reference the first slice (useful not to duplicate
  // vertices on more complex objects than just two triangles).
  let indexed_triangles = surface
    .new_tess()
    .set_vertices(&TRI_VERTICES[..])
    .set_indices(&TRI_INDICES[..])
    .set_mode(Mode::Triangle)
    .build()
    .unwrap();

  //// The back buffer, which we will make our render into (we make it mutable so that we can change
  //// it whenever the window dimensions change).
  let mut back_buffer = surface.back_buffer().unwrap();
  let mut resize = false;
  let mut points: Vec<(f64, f64)> = Vec::new();
  let mut left_button_pressed = false;

  'app: loop {
    let mut cursor_position: (f64, f64) = (-1.0, -1.0);

    // For all the events on the surface.
    surface.window.glfw.poll_events();
    for (_, event) in glfw::flush_messages(&surface.events_rx) {
      match event {
        // If we close the window or press escape, quit the main loop (i.e. quit the application).
        WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => break 'app,

        // Handle window resizing.
        WindowEvent::FramebufferSize(..) => {
          resize = true;
        }

        // Get cursor position
        WindowEvent::CursorPos(x, y) => {
            cursor_position = (x, y);
        }

        // Get mouse buttons
        WindowEvent::MouseButton(button, action, _modifiers) => {
            if button != MouseButton::Button1 {
                continue;
            }

            match action {
                Action::Press => left_button_pressed = true,
                Action::Release => left_button_pressed = false,
                _ => (),
            }
        }

        _ => (),
      }
    }

    //println!("{:?}, {:?}", left_button_pressed, cursor_position);
    if left_button_pressed && cursor_position != (-1.0, -1.0) {
        points.push(cursor_position);
    }

    if resize {
      // Simply ask another backbuffer at the right dimension (no allocation / reallocation).
      back_buffer = surface.back_buffer().unwrap();
      resize = false;
    }

    // Create a new dynamic pipeline that will render to the back buffer and must clear it with
    // pitch black prior to do any render to it.
    let render = surface
      .new_pipeline_gate()
      .pipeline(
        &back_buffer,
        &PipelineState::default(),
        |_, mut shd_gate| {
          // Start shading with our program.
          shd_gate.shade(&mut program, |_, _, mut rdr_gate| {
            // Start rendering things with the default render state provided by luminance.
            rdr_gate.render(&RenderState::default(), |mut tess_gate| {
              tess_gate.render(&indexed_triangles)
            })
          })
        },
      )
      .assume();

    // Finally, swap the backbuffer with the frontbuffer in order to render our triangles onto your
    // screen.
    if render.is_ok() {
      surface.window.swap_buffers();
      println!("{:?}", points);
    } else {
      break 'app;
    }
  }
}
