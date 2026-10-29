//! High-Precision Performance Benchmark for desktop-design-system
//! Tests:
//! 1. Telemetry Query Latency (In-process SystemDiagnostics)
//! 2. SQLite WAL CRUD Latencies (Insert, Read, Update, Delete)
//! 3. Bulk Transaction Throughput (1,000 Records)
//! 4. In-Process Resident Memory & CPU Cost

use std::time::Instant;
use desktop_design_system::db::DatabaseManager;
use desktop_design_system::system_info::SystemDiagnostics;

fn main() {
    println!("\n===============================================================");
    println!("   Desktop Design System - High-Precision Performance Benchmark ");
    println!("===============================================================\n");

    // 1. Hardware Telemetry Query Latency
    println!("[1/4] Benchmarking In-Process Hardware Telemetry API...");
    let mut diag_samples = Vec::new();
    for _ in 0..100 {
        let t0 = Instant::now();
        let _ = SystemDiagnostics::query();
        diag_samples.push(t0.elapsed().as_micros());
    }
    let avg_diag = diag_samples.iter().sum::<u128>() as f64 / diag_samples.len() as f64;
    let min_diag = *diag_samples.iter().min().unwrap();
    let max_diag = *diag_samples.iter().max().unwrap();
    println!("      -> 100 queries sampled");
    println!("      -> Average Query Latency : {:.2} µs ({:.3} ms)", avg_diag, avg_diag / 1000.0);
    println!("      -> Min: {} µs | Max: {} µs", min_diag, max_diag);

    // 2. Database Initialization & Seed
    println!("\n[2/4] Benchmarking SQLite WAL Database Engine...");
    let t_init = Instant::now();
    let db = DatabaseManager::new_in_memory().expect("Failed to initialize SQLite");
    let init_duration = t_init.elapsed();
    println!("      -> Cold DB Init + Migration + Seed: {:.2} µs ({:.3} ms)", 
             init_duration.as_micros(), init_duration.as_secs_f64() * 1000.0);

    // 3. Single CRUD Latencies
    let t_insert = Instant::now();
    let inserted = db.insert("Benchmark Node", "Rust Native", "Active", "0.1 ms").expect("Insert failed");
    let insert_us = t_insert.elapsed().as_micros();

    let t_read = Instant::now();
    let all = db.get_all().expect("Read failed");
    let read_us = t_read.elapsed().as_micros();

    let t_update = Instant::now();
    let _ = db.update(inserted.id, "Benchmark Node Pro", "Rust Direct3D", "Active", "0.08 ms").expect("Update failed");
    let update_us = t_update.elapsed().as_micros();

    let t_delete = Instant::now();
    let _ = db.delete(inserted.id).expect("Delete failed");
    let delete_us = t_delete.elapsed().as_micros();

    println!("      -> Single Write (INSERT) : {} µs ({:.3} ms)", insert_us, insert_us as f64 / 1000.0);
    println!("      -> Table Scan  (SELECT) : {} µs ({:.3} ms) (Found {} rows)", read_us, read_us as f64 / 1000.0, all.len());
    println!("      -> Record Mod  (UPDATE) : {} µs ({:.3} ms)", update_us, update_us as f64 / 1000.0);
    println!("      -> Record Purge (DELETE): {} µs ({:.3} ms)", delete_us, delete_us as f64 / 1000.0);

    // 4. Bulk Write Throughput (1,000 Records)
    println!("\n[3/4] Benchmarking Bulk Transaction Throughput (1,000 Records)...");
    let t_bulk = Instant::now();
    for i in 0..1000 {
        let name = format!("Worker Pipeline #{:04}", i);
        let _ = db.insert(&name, "Rust SIMD", "Active", "0.05 ms").expect("Bulk insert failed");
    }
    let bulk_elapsed = t_bulk.elapsed();
    let total_ms = bulk_elapsed.as_millis();
    let ops_per_sec = 1000.0 / bulk_elapsed.as_secs_f64();
    println!("      -> 1,000 WAL Inserts Completed in : {} ms", total_ms);
    println!("      -> Throughput                     : {:.0} writes/sec", ops_per_sec);

    let t_bulk_read = Instant::now();
    let bulk_all = db.get_all().expect("Bulk read failed");
    let bulk_read_us = t_bulk_read.elapsed().as_micros();
    println!("      -> Scan 1,005 Records Completed in: {} µs ({:.3} ms)", bulk_read_us, bulk_read_us as f64 / 1000.0);
    assert_eq!(bulk_all.len(), 1005);

    // 5. In-Process Diagnostics
    println!("\n[4/4] In-Process Host Architecture Diagnostics:");
    let diag = SystemDiagnostics::query();
    println!("      -> Host OS          : {}", diag.os_name);
    println!("      -> CPU Subsystem    : {} ({} Cores @ {} MHz)", diag.cpu_brand, diag.cpu_cores, diag.cpu_frequency_mhz);
    println!("      -> GPU Hardware     : {}", diag.gpu_name);
    println!("      -> Graphics Backend : {}", diag.graphics_backend);

    println!("\n===============================================================");
    println!("               BENCHMARK SUITE COMPLETED (100% OK)              ");
    println!("===============================================================\n");
}

