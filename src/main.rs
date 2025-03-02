use std::{default, thread, time};
use std::str;
use arraydeque::ArrayDeque;
use crossterm::{execute, terminal::{Clear, ClearType}, cursor::MoveTo};
use sysinfo::System;
use winping::{Buffer, Pinger};
use std::io::stdout;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};


struct PingResult {
    ip: String,
    last_ping: u32,
    max_ping: u32,
    min_ping: u32,
    ping_history: ArrayDeque<u32, 1000>,
    error_count: u32,
}

impl PingResult {
    fn new(ip: String) -> Self {
        Self {
            ip,
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

    fn set_ip(&mut self, ip: String) {
        if ip != self.ip {
            self.ip = ip;
            self.last_ping = 0;
            self.max_ping = 0;
            self.min_ping = 999999999;
            self.ping_history.clear();
            self.error_count = 0;
        }
    }
}

impl default::Default for PingResult {
    fn default() -> Self {
        Self {
            ip: String::new(),
            last_ping: 0,
            max_ping: 0,
            min_ping: 0,
            ping_history: ArrayDeque::new(),
            error_count: 0,
        }
    }
}

    impl std::fmt::Display for PingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IP: {}\nПинг: {}\nСредний пинг: {}\nМаксимальный пинг: {}\nМинимальный пинг: {}\nКоличество ошибок: {}",
            self.ip, self.last_ping, self.get_avg_ping(), self.max_ping, self.min_ping, self.error_count
        )
    }
}

fn main() {
    let mut ping_results = PingResult::default();
    let sleep_duration = time::Duration::from_secs(2);
    loop {
        let pid = match find_process_pids("ffxiv_dx11") {
            Some(pid) => pid,
            None => {
                display_result(&"Процесс не найден");
                thread::sleep(sleep_duration);
                continue;
            }
        };
        match find_game_servers(&pid) {
            Some(ip) => ping_results.set_ip(ip),
            None => {
                display_result(&"Отсутствует подключение к серверу игры");
                thread::sleep(sleep_duration);
                continue;
            }
        };
        match check_ping(ping_results.ip.parse().unwrap()) {
            Some(ping) => ping_results.update_ping(ping),
            None => ping_results.update_error(),
        };
        display_result(&ping_results);
        thread::sleep(sleep_duration);
    }
}

fn find_process_pids(name: &str) -> Option<u32> {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.processes()
    .values()
    .filter(|proc| proc.name().to_lowercase().contains(name))
    .map(|proc| proc.pid().as_u32())
    .next()
}

fn find_game_servers(pid: &u32) -> Option<String> {
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
        }).next()
}

fn check_ping(dst: core::net::IpAddr) -> Option<u32> {
    let pinger = Pinger::new().unwrap();
    let mut buffer = Buffer::new();
    match pinger.send(dst, &mut buffer) {
        Ok(rtt) => Some(rtt),
        Err(_) => None,
    }
}

fn display_result(text: &dyn std::fmt::Display) {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0)).unwrap(); // Очищаем экран

    println!("Мониторинг серверов игры:");
    println!("-------------------------");
    println!("{}", text);
}
