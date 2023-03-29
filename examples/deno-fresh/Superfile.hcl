project = "deno-fresh"
services = [
  {
    "name" = "deno"
    "type" = "exec"
    "command" = "./dev.ts"
    "working_dir" = "."
    "description" = "Deno example app"
    "depends_on" = []
    "env" = {}
    "autostart" = true
    "autorestart" = false
    "namespace" = "deno_namespace"
    "stdout" = "/tmp/deno-stdout.log"
    "stderr" = "/tmp/deno-stderr.log"
    "port" = 8000
    flox = {
      "environment" = ".#deno-fresh"
    }
  }
]