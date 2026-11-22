use std::env;
use std::fs;

const HELP: &str = "\
apifuzz — генератор тестов из OpenAPI-спецификации

  apifuzz version
  apifuzz generate <openapi.json> [out.json]
  apifuzz demo
";

const DEMO_SPEC: &str = r#"{
  "openapi":"3.0.3",
  "info":{"title":"demo","version":"1.0.0"},
  "paths":{
    "/users/{id}":{"get":{"parameters":[
      {"name":"id","in":"path","required":true,"schema":{"type":"string"}},
      {"name":"page","in":"query","required":false,"schema":{"type":"integer"}}
    ],"responses":{"200":{},"404":{}}}},
    "/users":{"post":{"requestBody":{"content":{"application/json":{"schema":{
      "type":"object","properties":{
        "name":{"type":"string"},
        "role":{"type":"string","enum":["admin","user"]}
      },"required":["name"]}}}},"responses":{"201":{},"400":{}}}}
  }
}"#;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "help" {
        print!("{}", HELP);
        return;
    }
    match args[0].as_str() {
        "version" => println!("{}", apifuzz_core::VERSION),
        "demo" => match apifuzz_core::generate(DEMO_SPEC) {
            Ok(report) => println!("{}", report),
            Err(err) => {
                eprintln!("apifuzz: {}", err);
                std::process::exit(1);
            }
        },
        "generate" => {
            let Some(path) = args.get(1) else {
                eprintln!("нужен файл openapi.json");
                std::process::exit(2);
            };
            let spec = match fs::read_to_string(path) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("{}: {}", path, err);
                    std::process::exit(1);
                }
            };
            match apifuzz_core::generate(&spec) {
                Ok(report) => {
                    if let Some(out) = args.get(2) {
                        if let Err(err) = fs::write(out, &report) {
                            eprintln!("{}: {}", out, err);
                            std::process::exit(1);
                        }
                        println!("apifuzz: cases written to {}", out);
                    } else {
                        println!("{}", report);
                    }
                }
                Err(err) => {
                    eprintln!("apifuzz: {}", err);
                    std::process::exit(2);
                }
            }
        }
        other => {
            eprintln!("неизвестная команда: {}", other);
            print!("{}", HELP);
            std::process::exit(2);
        }
    }
}
