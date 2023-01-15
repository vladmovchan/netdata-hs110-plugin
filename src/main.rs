use core::time;
use netdata_plugin::{collector::Collector, Algorithm, Chart, ChartType, Dimension};
use serde::{Deserialize, Serialize};
use std::{env, error, fs::File, io, thread, time::Instant};
use tplink_hs1x0::HS110;

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    hosts: Vec<String>,
}

#[derive(Debug)]
struct Device<'a> {
    addr: &'a str,
    alias: String,
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

    let config_path = format!(
        "{dir}/hs110.conf",
        dir = env::var("NETDATA_USER_CONFIG_DIR").unwrap_or_else(|_| {
            let fallback = "/usr/local/etc/netdata".to_owned();
            eprintln_time_and_name!(
                "Warning: `NETDATA_USER_CONFIG_DIR` environment variable is not defined. \
                Using `{fallback}`."
            );
            fallback
        })
    );
    eprintln_time_and_name!("Info: Reading config file `{config_path}`...");
    let config: Config = serde_yaml::from_reader(File::open(config_path)?)?;
    if config.hosts.is_empty() {
        eprintln_time_and_name!("Error: at least one host has to be specified in the config");
        return Err("At least one host has to be specified in the config".into());
    }
    eprintln_time_and_name!(
        "Info: The following devices are going to be polled: {:?}",
        config.hosts
    );

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

    let devices = config
        .hosts
        .iter()
        .map(|addr| {
            let dimension_prefix = addr.replace('.', "_");
            let hs110 =
                HS110::new(addr).with_timeout(time::Duration::from_millis(delay * 1000 / 2));
            let alias = hs110.hostname().unwrap_or_else(|_| "<unknown>".to_owned());
            Device {
                addr,
                dimension_prefix,
                alias,
                hs110,
            }
        })
        .collect::<Vec<_>>();

    for (chart, index) in charts_and_indexes.iter() {
        collector.add_chart(chart)?;
        for Device {
            addr,
            alias,
            dimension_prefix,
            ..
        } in devices.iter()
        {
            let dimension = Dimension {
                id: &format!("{dimension_prefix}_{chart_name}", chart_name = chart.name),
                name: &format!("{alias} ({addr})"),
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
            alias,
            addr,
        } in devices.iter()
        {
            let emeter = match hs110.emeter_parsed() {
                Ok(res) => res,
                Err(e) => {
                    eprintln_time_and_name!(
                        "Warning: unable to obtain emeter values from {addr} [{alias}]: {e}"
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
                                    "Warning: unable to parse `{index}` value `{value}` obtained from {addr} [{alias}]"
                                );
                                0.0
                            }) as i64,
                        )?;
                    }
                    None => {
                        eprintln_time_and_name!(
                            "Warning: `{index}` is not available in emeter readings from {addr} [{alias}]"
                        );
                        continue;
                    }
                };
            }
        }

        thread::sleep(time::Duration::from_secs(delay).saturating_sub(start.elapsed()));
        for (chart, _) in charts_and_indexes.iter() {
            collector.commit_chart(chart.type_id).unwrap();
        }
    }
}
