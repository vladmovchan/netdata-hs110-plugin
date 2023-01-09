use core::time;
use netdata_plugin::{collector::Collector, Algorithm, Chart, ChartType, Dimension};
use std::{env, error, io, thread, time::Instant};
use tplink_hs1x0::HS110;

#[derive(Debug)]
struct Device<'a> {
    ip: &'a str,
    hostname: String,
    dimension_prefix: String,
    hs110: HS110,
}

macro_rules! eprintln_time_and_name {
    ($($arg:tt)*) => {
        let binary_path = env::args().next().unwrap_or_else(|| "".to_owned());
        let binary_name = match binary_path.rfind('/') {
            Some(pos) => binary_path[pos+1..].to_owned(),
            None => binary_path,
        };
        eprintln!(
            "{}: {}: {}",
            chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S"),
            binary_name,
            format!($($arg)*)
        );
    };
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let delay = env::args()
        .skip(1)
        .next()
        .unwrap_or_else(|| {
            eprintln_time_and_name!("Warning: delay has not been specified, using 1 sec delay");
            "1".to_string()
        })
        .parse::<u64>()
        .unwrap_or_else(|err| {
            eprintln_time_and_name!(
                "Warning: unable to parse specified delay ({err}), using 1 sec delay"
            );
            1
        });

    let mut writer = io::stdout();
    let mut collector = Collector::new(&mut writer);

    let charts_and_indexes = vec![
        (
            Chart {
                type_id: "Smartplugs.current",
                name: "Current",
                title: "Current",
                units: "amps",
                familiy: "current",
                context: "smartplugpower.current",
                charttype: Some(ChartType::line),
                priority: Some(92000),
                ..Default::default()
            },
            "current_ma",
        ),
        (
            Chart {
                type_id: "Smartplugs.voltage",
                name: "Voltage",
                title: "Voltage",
                units: "volts",
                familiy: "voltage",
                context: "smartplugpower.voltage",
                charttype: Some(ChartType::line),
                priority: Some(91000),
                ..Default::default()
            },
            "voltage_mv",
        ),
        (
            Chart {
                type_id: "Smartplugs.power",
                name: "Power",
                title: "Power",
                units: "watts",
                familiy: "power",
                context: "smartplugpower.power",
                charttype: Some(ChartType::area),
                priority: Some(90000),
                ..Default::default()
            },
            "power_mw",
        ),
        (
            Chart {
                type_id: "Smartplugs.total-consumption",
                name: "Total",
                title: "Total consumption",
                units: "watt-hours",
                familiy: "consumption",
                context: "smartplugpower.total",
                charttype: Some(ChartType::line),
                priority: Some(94000),
                ..Default::default()
            },
            "total_wh",
        ),
    ];

    let ips = vec![
        "192.168.0.191",
        "192.168.0.156",
        "192.168.0.155",
        "192.168.0.123",
        "192.168.0.102",
        "192.168.0.122",
    ];

    let devices = ips
        .iter()
        .map(|ip| {
            let dimension_prefix = ip.replace('.', "_");
            let hs110 = HS110::new(ip.to_string());
            let hostname = hs110.hostname().unwrap_or_else(|_| "<unknown>".to_owned());
            Device {
                ip,
                dimension_prefix,
                hostname,
                hs110,
            }
        })
        .collect::<Vec<_>>();

    for (chart, index) in charts_and_indexes.iter() {
        collector.add_chart(chart)?;
        for Device {
            ip,
            hostname,
            dimension_prefix,
            ..
        } in devices.iter()
        {
            let dimension = Dimension {
                id: &format!("{dimension_prefix}_{chart_name}", chart_name = chart.name),
                name: &format!("{hostname} ({ip})"),
                algorithm: Some(Algorithm::absolute),
                divisor: match index.find("_m") {
                    Some(_) => Some(1000),
                    None => Some(1),
                },
                ..Default::default()
            };
            collector.add_dimension(chart.type_id, &dimension)?;
        }
    }

    loop {
        let start = Instant::now();
        for Device {
            hs110,
            dimension_prefix,
            hostname,
            ip,
        } in devices.iter()
        {
            let emeter = match hs110.emeter_parsed() {
                Ok(res) => res,
                Err(e) => {
                    eprintln_time_and_name!(
                        "Warning: unable to obtain emeter values from {ip} [{hostname}]: {e}"
                    );
                    continue;
                }
            };
            for (chart, index) in charts_and_indexes.iter() {
                match emeter.get(index) {
                    Some(value) => {
                        let dimension_id =
                            format!("{dimension_prefix}_{chart_name}", chart_name = chart.name);
                        collector.prepare_value(
                            chart.type_id,
                            &dimension_id,
                            value.as_f64().unwrap_or_else(|| {
                                eprintln_time_and_name!(
                                    "Warning: unable to parse `{index}` value `{value}` obtained from {ip} [{hostname}]"
                                );
                                0.0
                            }) as i64,
                        )?;
                    }
                    None => {
                        eprintln_time_and_name!(
                            "Warning: `{index}` is not available in emeter readings from {ip} [{hostname}]"
                        );
                        continue;
                    }
                };
            }
        }
        for (chart, _) in charts_and_indexes.iter() {
            collector.commit_chart(chart.type_id).unwrap();
        }

        thread::sleep(time::Duration::from_secs(delay).saturating_sub(start.elapsed()));
    }
}
