use netdata_plugin::{collector::Collector, Algorithm, Chart, ChartType, Dimension};
use std::{env, error, io};
use tplink_hs1x0::HS110;

fn main() -> Result<(), Box<dyn error::Error>> {
    let delay = env::args()
        .skip(1)
        .next()
        .unwrap_or_else(|| {
            eprintln!("Warning: delay has not been specified, using 1 sec delay");
            "1".to_string()
        })
        .parse::<u64>()
        .unwrap_or_else(|err| {
            eprintln!("Warning: unable to parse specified delay ({err}), using 1 sec delay");
            1
        });

    let mut writer = io::stdout();
    let mut collector = Collector::new(&mut writer);

    let charts = vec![
        Chart {
            type_id: "power.cur",
            name: "Current",
            title: "Currenttitle",
            units: "amps",
            familiy: "devices",
            context: "smartplugpower.current",
            charttype: Some(ChartType::line),
            ..Default::default()
        },
        Chart {
            type_id: "power.volt",
            name: "Voltage",
            title: "Voltagetitle",
            units: "volts",
            familiy: "devices",
            context: "smartplugpower.voltage",
            charttype: Some(ChartType::line),
            ..Default::default()
        },
        Chart {
            type_id: "power.pow",
            name: "Power",
            title: "Powertitle",
            units: "watts",
            familiy: "devices",
            context: "smartplugpower.power",
            charttype: Some(ChartType::area),
            ..Default::default()
        },
        Chart {
            type_id: "power.tot",
            name: "Total",
            title: "Totaltitle",
            units: "watt-hours",
            familiy: "devices",
            context: "smartplugpower.total",
            charttype: Some(ChartType::line),
            ..Default::default()
        },
    ];

    let hosts = vec![
        "192.168.0.191",
        "192.168.0.156",
        "192.168.0.155",
        "192.168.0.123",
        "192.168.0.102",
        "192.168.0.122",
    ];
    let hs110 = HS110::new("192.168.0.156".to_string());

    let dimensions = vec![
        Dimension {
            id: charts[0].name,
            name: "dim name (hostname)",
            algorithm: Some(Algorithm::absolute),
            ..Default::default()
        },
        Dimension {
            id: charts[1].name,
            name: "dim name (hostname)",
            algorithm: Some(Algorithm::absolute),
            ..Default::default()
        },
        Dimension {
            id: charts[2].name,
            name: "dim name (hostname)",
            algorithm: Some(Algorithm::absolute),
            ..Default::default()
        },
        Dimension {
            id: charts[3].name,
            name: "dim name (hostname)",
            algorithm: Some(Algorithm::absolute),
            ..Default::default()
        },
    ];

    for (chart, dimension) in charts.iter().zip(dimensions.iter()) {
        collector.add_chart(chart)?;
        collector.add_dimension(chart.type_id, dimension)?;
    }

    loop {
        let emeter = hs110.emeter_parsed()?;
        match emeter.get("current_ma").or_else(|| emeter.get("current")) {
            Some(value) => {
                collector.prepare_value(
                    charts[0].type_id,
                    dimensions[0].id,
                    value.as_f64().unwrap() as i64,
                )?;
            }
            None => {
                panic!("NONE0");
            }
        };
        match emeter.get("voltage_mv").or_else(|| emeter.get("voltage")) {
            Some(voltage) => {
                collector.prepare_value(
                    charts[1].type_id,
                    dimensions[1].id,
                    voltage.as_f64().unwrap() as i64,
                )?;
            }
            None => {
                panic!("NONE1");
            }
        };

        match emeter.get("power_mw").or_else(|| emeter.get("power")) {
            Some(power) => {
                collector.prepare_value(
                    charts[2].type_id,
                    dimensions[2].id,
                    power.as_f64().unwrap() as i64,
                )?;
            }
            None => {
                panic!("NONE2");
            }
        };

        match emeter.get("total_wh").or_else(|| emeter.get("total")) {
            Some(value) => {
                collector.prepare_value(
                    charts[3].type_id,
                    dimensions[3].id,
                    value.as_f64().unwrap() as i64,
                )?;
            }
            None => {
                panic!("NONE3");
            }
        };
        collector.commit_chart(charts[0].type_id).unwrap();
        collector.commit_chart(charts[1].type_id).unwrap();
        collector.commit_chart(charts[2].type_id).unwrap();
        collector.commit_chart(charts[3].type_id).unwrap();

        std::thread::sleep(std::time::Duration::from_secs(delay));
    }
}
