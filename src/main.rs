#[macro_use]
extern crate windows_service;

mod api;
mod server;
mod warp_utils;

use std::thread::sleep;
use std::ffi::OsString;
use std::time::{Duration, Instant};
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
    ServiceType, ServiceInfo, ServiceStartType, ServiceErrorControl, ServiceAccess, Service,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;
use windows_service::service_manager::{ServiceManagerAccess, ServiceManager};

const SERVICE_NAME: &str = "MinecraftManager";
const DISPLAY_NAME: &str = "Minecraft Manager";
const DESCRIPTION: &str = "Um servidor http que permite que usuários criem, apaguem, liguem e desliguem instâncias de servidores de minecraft";
const HELP_MESSAGE: &str =
r"comandos válidos são:
  install   - instalar serviço
  uninstall - desinstalar serviço
  start     - inicia o serviço
  stop      - para o serviço
  status    - mostra o status do serviço
  run       - executar imediatamente, sem ser um serviço";
fn main() -> Result<(), windows_service::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        return Ok(());
    }
    match args[1].as_str() {
        "i" | "install" => install_service(),
        "u" | "uninstall" => uninstall_service(),
        "start" => start_service(),
        "stop" => stop_service(),
        "s" | "status" => status_service(),
        "r" | "run" => server::serve(None),
        "h" | "help" => println!("{}", HELP_MESSAGE),
        _ => println!("comando desconhecido: {}\n{}", args[1], HELP_MESSAGE),
    }
    Ok(())
}

define_windows_service!(ffi_service_main, rust_service_main);

fn rust_service_main(arguments: Vec<OsString>) {
    if let Err(_e) = run_service(arguments) {
        // Handle error in some way.
    }
}

fn run_service(_arguments: Vec<OsString>) -> windows_service::Result<()> {
    let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel();
    let mut shutdown_sender = Some(shutdown_sender);
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                if let Some(sender) = shutdown_sender.take() {
                    sender.send(()).unwrap();
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => {
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler
    let status_handle = service_control_handler::register("mc_manager", event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    server::serve(Some(shutdown_receiver));

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())

}

fn install_service() {
    use windows_sys::Win32::Foundation::ERROR_SERVICE_EXISTS;
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access).expect("ServiceManager");

    // This example installs the service defined in `examples/ping_service.rs`.
    // In the real world code you would set the executable path to point to your own binary
    // that implements windows service.
    let service_binary_path = std::env::current_exe().expect("current_exe");

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    match service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG) {
        Ok(service) => {
            service.set_description(DESCRIPTION).expect("set_description");
            println!("O serviço foi instalado");
        }
        Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(ERROR_SERVICE_EXISTS as i32) => {
            println!("O serviço já está instalado");
        }
        result => {
            result.expect("create_service");
        }
    }
    
}

fn uninstall_service() {
    use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access).expect("ServiceManager");

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = match service_manager.open_service(SERVICE_NAME, service_access) {
        Ok(service) => service,
        Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) => {
            println!("O serviço não está instalado");
            return;
        },
        result => {
            result.expect("open_service");
            return;
        },
    };

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete().expect("delete");
    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status().expect("query_status").current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop().expect("stop");
    }
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("O serviço foi desinstalado");
                return;
            }
        }
        sleep(Duration::from_secs(1));
    }
    println!("O serviço foi marcado para ser apagado");
}

fn start_service() {
    use std::ffi::OsStr;
    if let Some(service) = open_service(ServiceAccess::START) {
        service.start::<&OsStr>(&[]).expect("start");
        println!("O serviço está iniciando");
    }
}

fn stop_service() {
    if let Some(service) = open_service(ServiceAccess::STOP) {
        service.stop().expect("stop");
        println!("O serviço está parando");
    }
}

fn status_service() {
    if let Some(service) = open_service(ServiceAccess::QUERY_STATUS) {
        let status = service.query_status().expect("query_status").current_state;
        match status {
            ServiceState::Stopped => println!("O serviço está parado"),
            ServiceState::StartPending => println!("O serviço está iniciando"),
            ServiceState::StopPending => println!("O serviço está parando"),
            ServiceState::Running => println!("O serviço está executando"),
            ServiceState::ContinuePending => println!("O serviço está despausando"),
            ServiceState::PausePending => println!("O serviço está pausando"),
            ServiceState::Paused => println!("O serviço está pausado"),
        }
    }
}

fn open_service(service_access: ServiceAccess) -> Option<Service> {
    use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access).expect("ServiceManager");

    match service_manager.open_service(SERVICE_NAME, service_access) {
        Ok(service) => Some(service),
        Err(windows_service::Error::Winapi(error)) if error.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) => {
            println!("O serviço não esta instalado");
            None
        },
        result => {
            result.expect("open_service");
            None
        },
    }
}
