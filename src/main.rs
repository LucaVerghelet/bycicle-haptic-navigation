use axum::{
    extract::State,
    routing::{get, post},
    Router,
    Json,
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use rppal::gpio::{Gpio, OutputPin};
use regex::Regex;

const PIN_IN1: u8 = 17; 
const PIN_IN2: u8 = 27; 
const PIN_IN3: u8 = 22; 
const PIN_IN4: u8 = 23; 

struct MotorController {
    motor_stanga_1: OutputPin,
    motor_stanga_2: OutputPin,
    motor_dreapta_1: OutputPin,
    motor_dreapta_2: OutputPin,
}

impl MotorController {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let gpio = Gpio::new()?;
        let mut m_s_1 = gpio.get(PIN_IN1)?.into_output();
        let mut m_s_2 = gpio.get(PIN_IN2)?.into_output();
        let mut m_d_1 = gpio.get(PIN_IN3)?.into_output();
        let mut m_d_2 = gpio.get(PIN_IN4)?.into_output();

        m_s_1.set_low(); m_s_2.set_low();
        m_d_1.set_low(); m_d_2.set_low();

        Ok(Self {
            motor_stanga_1: m_s_1, motor_stanga_2: m_s_2,
            motor_dreapta_1: m_d_1, motor_dreapta_2: m_d_2,
        })
    }

    fn vibreaza_stanga(&mut self) {
        self.motor_stanga_1.set_high();
        std::thread::sleep(Duration::from_millis(1000)); 
        self.motor_stanga_1.set_low();
    }

    fn vibreaza_dreapta(&mut self) {
        self.motor_dreapta_1.set_high();
        std::thread::sleep(Duration::from_millis(1000));
        self.motor_dreapta_1.set_low();
    }
}

struct AppState {
    motors: Mutex<MotorController>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let motor_controller = match MotorController::new() {
        Ok(mc) => mc,
        Err(e) => {
            eprintln!("EROARE CRITICA GPIO: {}", e);
            return;
        }
    };

    let state = Arc::new(AppState {
        motors: Mutex::new(motor_controller),
    });

    let app = Router::new()
        .route("/", get(root))
        .route("/notify", post(receive_notification))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server pornit pe http://{}:3000", addr.ip());
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> Json<Value> {
    Json(json!({ "status": "online", "mode": "final_turn_only" }))
}

async fn receive_notification(
    State(state): State<Arc<AppState>>, 
    Json(payload): Json<Value>
) {
    let app_name = payload.get("app").and_then(|v| v.as_str()).unwrap_or("Unknown");
    
    if app_name != "com.google.android.apps.maps" {
        return;
    }

    let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
    
    println!("Notificare Maps: '{}'", title);

    let re_distanta = Regex::new(r"^\d").unwrap();

    if re_distanta.is_match(title) {
        return; 
    }

    let title_lower = title.to_lowercase();
    let mut controller = state.motors.lock().await;

    if title_lower.contains("right") || title_lower.contains("dreapta") {
        println!("DREAPTA"); 
        controller.vibreaza_dreapta();
        
    } else if title_lower.contains("left") || title_lower.contains("stânga") || title_lower.contains("stanga") {
        println!("STANGA"); 
        controller.vibreaza_stanga();
    }
}