#[macro_use] extern crate clap;
extern crate yaml_rust;
extern crate rusoto_core;
extern crate rusoto_ssm;
extern crate crossbeam;

use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use std::{thread, time};
use clap::{App, AppSettings};
use yaml_rust::{yaml};
use rusoto_core::{Region};
use rusoto_ssm::{Ssm, SsmClient, SendCommandRequest, GetCommandInvocationRequest, ListCommandInvocationsRequest, Target};

fn parse_region(s: &str) -> Result<Region, Region> {
    let v : &str = &s.to_lowercase();
    match v {
        "ap-northeast-1" | "apnortheast1" => Ok(Region::ApNortheast1),
        "ap-northeast-2" | "apnortheast2" => Ok(Region::ApNortheast2),
        "ap-south-1" | "apsouth1" => Ok(Region::ApSouth1),
        "ap-southeast-1" | "apsoutheast1" => Ok(Region::ApSoutheast1),
        "ap-southeast-2" | "apsoutheast2" => Ok(Region::ApSoutheast2),
        "ca-central-1" | "cacentral1" => Ok(Region::CaCentral1),
        "eu-central-1" | "eucentral1" => Ok(Region::EuCentral1),
        "eu-west-1" | "euwest1" => Ok(Region::EuWest1),
        "eu-west-2" | "euwest2" => Ok(Region::EuWest2),
        "eu-west-3" | "euwest3" => Ok(Region::EuWest3),
        "sa-east-1" | "saeast1" => Ok(Region::SaEast1),
        "us-east-1" | "useast1" => Ok(Region::UsEast1),
        "us-east-2" | "useast2" => Ok(Region::UsEast2),
        "us-west-1" | "uswest1" => Ok(Region::UsWest1),
        "us-west-2" | "uswest2" => Ok(Region::UsWest2),
        "us-gov-west-1" | "usgovwest1" => Ok(Region::UsGovWest1),
        "cn-north-1" | "cnnorth1" => Ok(Region::CnNorth1),
        "cn-northwest-1" | "cnnorthwest1" => Ok(Region::CnNorthwest1),
        _s => Err(Region::UsEast1),
    }
}

fn get_command (a: &str, b: &str) -> yaml_rust::Yaml {
    let mut file = File::open(a)
        .expect("failed to open command inventory file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("failed to read command inventory file");    
    
    let command_yaml = yaml::YamlLoader::load_from_str(&contents).unwrap();
    let commands = command_yaml[0].clone();
    let command = commands[b].clone();
    
    return command;
}

fn get_parameters (a: yaml_rust::Yaml) -> HashMap<std::string::String, std::vec::Vec<std::string::String>> {
    let mut m: HashMap<std::string::String, std::vec::Vec<std::string::String>> = HashMap::new();
    let parameters = &a["parameters"];
    let mut commands: std::vec::Vec<std::string::String> = Vec::new();
    for i in parameters["commands"].clone().into_vec().unwrap() {
        commands.push(i.into_string().unwrap());
    }
    m.insert("workingDirectory".to_string(), vec![parameters["workingDirectory"].clone().into_string().unwrap()]);
    m.insert("executionTimeout".to_string(), vec![parameters["executionTimeout"].clone().into_string().unwrap()]);
    m.insert("commands".to_string(), commands);
    return m;
}

fn get_targets (a: yaml_rust::Yaml) -> std::vec::Vec<rusoto_ssm::Target> {
    let mut v = Vec::new();
    for i in a["targets"].clone().into_vec().unwrap() {
        let key = i["key"].clone().into_string().unwrap();
        let value = vec![i["values"].clone().into_string().unwrap()];
        v.push(Target{
                key: Some(key),
                values: Some(value)
            }
        );
    }
    return v;
}

fn get_invocations (c: rusoto_ssm::SsmClient, ci: std::string::String) -> std::vec::Vec<rusoto_ssm::CommandInvocation> {
    let invo_req = ListCommandInvocationsRequest {
        command_id: Some(ci.to_string()),
        ..Default::default()
    };
    let resp = c.list_command_invocations(invo_req).sync().ok().unwrap();
    return resp.command_invocations.unwrap();
}

fn run_command (c: rusoto_ssm::SsmClient, y: yaml_rust::Yaml, b: std::string::String) -> rusoto_ssm::Command {
    let run_req = SendCommandRequest {
        document_name: "AWS-RunShellScript".to_string(),
        max_concurrency: Some(b),
        parameters: Some(get_parameters(y.clone())),
        targets: Some(get_targets(y.clone())),
        ..Default::default()
    };
    let resp = c.send_command(run_req).sync().ok().unwrap();
    println!("AWS Systems Manager: Command submitted successfully");
    return resp.command.unwrap();
}

fn wait_for_command (c: rusoto_ssm::SsmClient, ci: std::string::String,  i: rusoto_ssm::CommandInvocation) {
    let wait_req = GetCommandInvocationRequest {
        command_id: ci.to_string(),
        instance_id: i.instance_id.unwrap(),
        ..Default::default()
    };

    loop {
        let resp = c.get_command_invocation(wait_req.clone()).sync().ok().unwrap();
        println!("Instance: {} Status: {} Duration: {}",
            resp.instance_id.unwrap(), 
            resp.status_details.unwrap(),
            resp.execution_elapsed_time.unwrap()
        );

        thread::sleep(time::Duration::from_secs(2));

        match resp.status.unwrap().as_str() {
            "Success" => break,
            "Cancelled" => break,
            "TimedOut" => break,
            "Failed" => break,
            _ => continue,
        }
    }
}

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).setting(AppSettings::ArgRequiredElseHelp).get_matches();

    let batch = matches.value_of("batch").unwrap_or("1").to_string();

    let command_file = matches.value_of("inventory").unwrap();
    let command_name = matches.value_of("execute").unwrap();
    let command = get_command(command_file, command_name);

    // NOTE: Rusoto behavior is to take malformed/non-existant
    // regions and default to `us-east-1`.  I am mirroring that behavior here.
    let region_val = matches.value_of("region").unwrap_or("us-east-1");
    let region_result = parse_region(region_val);
    let region = match region_result {
        Ok(v) => v,
        Err(e) => e,
    };

    let ssm_client = SsmClient::new(region.clone());
    let result = run_command(ssm_client.clone(), command, batch);
    let command_id = result.command_id.unwrap();
    
    // Rust is faster than AWS so we need to wait for the command 
    // to register on the backend.
    thread::sleep(time::Duration::from_secs(2));

    let invocations = get_invocations(ssm_client.clone(), command_id.clone());
    for invocation in invocations {
        crossbeam::scope(|scope| {
            scope.spawn(|_| {
                wait_for_command(ssm_client.clone(), command_id.clone(), invocation);
            });
        }).unwrap();
    }
}