# dirk

`dirk` is a tool to orchestrate command execution via the AWS Systems Manager Run Command feature.

## Overview

`dirk` lets you describe commands you'd like to execute via AWS Systems Manager's Run Command feature with YAML files.  At the moment, AWS does not support re-running previously executed commands.  They are meant to be one-off, unscheduled actions and the operator then needs to rebuild the command invocation request by hand (either using the AWS Console or the awscli command line tool).  This is cumbersome in my opinion. I created this tool to help make command invocation requests easier to run on the fly and easier to codify into version control.

## Download

You can download a prebuilt binary [here](https://github.com/slapula/dirk/releases).

## Help

```
USAGE:
    dirk [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --batch <INTEGER>       Number of instances to execute command concurrently (default: 1)
    -e, --execute <STRING>      Command to execute
    -i, --inventory <FILE>      Inventory file for commands
    -r, --region <STRING>       AWS Region (Default: 'us-east-1')
```

### Getting Started

Using `dirk` is easy.  First, you need to create an inventory file for the command(s) you want to run.  The file name you use will be the value you feed into `-i` or `--inventory` option.  This file can contain as many YAML stanzas as you like as long as the name of each stanza is unique.  Each stanza should look like the example below:
```
---
test:
 batch: 1
 parameters:
  workingDirectory: ""
  executionTimeout: "3600"
  commands:
    - "#!/bin/bash"
    - "echo Hello World"
 targets:
  - key: tag:app
    values: "demo"
  - key: tag:env
    values: "test"
```
These key/value pairs line up with specific parameters in the AWS Systems Manager API.  To read more on what you can use for each value, please check out the [AWS documentation](https://docs.aws.amazon.com/systems-manager/latest/APIReference/API_SendCommand.html).

The above example allows you to run `dirk -i commands.yml -e test` which will execute those bash commands on instances with the `app:demo` tag and the `env:test` tag.

## NOTE

I'm new to Rust so if you would like to report a bug, submit a feature request, or even contribute please be my guest!