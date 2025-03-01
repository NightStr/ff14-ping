use std::ops::Index;
use std::{thread, time};
use std::str;
use arraydeque::ArrayDeque;
use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveTo};
use sysinfo::System;
use winping::{Buffer, Pinger};
use std::io::stdout;
use std::net::IpAddr;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};


struct PingResult {
    last_ping: u32,
    max_ping: u32,
    min_ping: u32,
    ping_history: ArrayDeque<u32, 1000>,
    error_count: u32,
}

impl PingResult {
    fn new() -> Self {
        Self {
            last_ping: 0,
            ping_history: ArrayDeque::new(),
            error_count: 0,
            max_ping: 0,
            min_ping: 999999999,
        }
    }

    fn update_ping(&mut self, ping: u32) {
        self.last_ping = ping;
        if ping > self.max_ping {
            self.max_ping = ping;
        }
        if ping < self.min_ping {
            self.min_ping = ping;
        }
        match self.ping_history.push_back(ping) {
            Ok(_) => {}
            Err(_) => {
                self.ping_history.pop_front();
                let _ = self.ping_history.push_back(ping);
            }
        }
    }

    fn update_error(&mut self) {
        self.error_count += 1;
    }

    fn get_avg_ping(&self) -> u32 {
        if self.ping_history.is_empty() {
            0
        } else {
            self.ping_history.iter().sum::<u32>() / self.ping_history.len() as u32
        }
    }
}

fn main() {
    let process_name = "ffxiv_dx11"; // Укажите игру
    let refresh_interval = time::Duration::from_secs(2); // Интервал обновления (секунды)
    let pid = find_process_pids(process_name).expect("Процесс не найден");
    let mut ping_results = PingResult::new();
    loop {
        let ip = find_game_servers(&pid);
        match check_ping(ip.parse().unwrap()) {
            Ok(ping) => {
                ping_results.update_ping(ping);
            }
            Err(_) => {
                ping_results.update_error();
            }
            
        };
        display_ping(&ip, &ping_results);
        thread::sleep(refresh_interval);
    }
}

fn find_process_pids(name: &str) -> Option<u32> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let pids: Vec<u32> = sys.processes()
        .values()
        .filter(|proc| proc.name().to_lowercase().contains(name))
        .map(|proc| proc.pid().as_u32())
        .collect();
    if pids.is_empty() {
        None
    } else {
        Some(pids[0])
    }
}

fn find_game_servers(pid: &u32) -> String {
    let sockets_info = get_sockets_info(AddressFamilyFlags::IPV4, ProtocolFlags::TCP).unwrap();
    
    sockets_info
        .into_iter()
        .filter(|socket_info| socket_info.associated_pids.contains(pid))
        .filter_map(|si| {
            if let ProtocolSocketInfo::Tcp(tcp_si) = si.protocol_socket_info {
                Some(tcp_si.remote_addr.to_string()) // Берём remote_addr
            } else {
                None
            }
        }).collect::<Vec<String>>().index(0).to_string()
}

// 3. Проверить пинг до сервера
fn check_ping(host: IpAddr) -> Result<u32, String> {
    let pinger = Pinger::new().unwrap();
    let mut buffer = Buffer::new();
    match pinger.send(host, &mut buffer) {
        Ok(rtt) => Ok(rtt),
        Err(err) => Err(format!("{}", err)),
    }
}

// 4. Отображение пинга
fn display_ping(ip: &String, ping_result: &PingResult) {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap(); // Очищаем экран

    println!("Мониторинг серверов игры:");
    println!("-------------------------");
    println!("Сервер: {}\nПинг: {}\nСреднее: {}\nМаксимальный: {}\nМинимальный: {}\nКоличество ошибок: {}", ip, ping_result.last_ping, ping_result.get_avg_ping(),ping_result.max_ping, ping_result.min_ping, ping_result.error_count);
}
