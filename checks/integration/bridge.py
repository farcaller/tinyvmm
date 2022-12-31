import time
import json

start_all()

vmm.succeed('mkdir -p /run/systemd/network/')

vmm.wait_for_unit('tinyvmm.service')

create_bridge = {
  "apiVersion": "v1alpha1",
  "kind": "Bridge",
  "metadata": {
    "name": "vmbr0"
  },
  "spec": {
    "address": "10.10.0.1/24",
    "dnsZone": "vm.example.com",
    "dnsServer": "100.100.100.100"
  }
}

vmm.succeed(f"""
  curl -v --unix-socket /run/tinyvmm/sock http://localhost/api/v1/bridges -X POST -H 'Content-Type: application/json' -d '{json.dumps(create_bridge)}'
""")

time.sleep(15)

vmm.succeed("""
  test "$(
    ip -j a s vmbr0 |
    jq -r '.[0].addr_info[] | select(.family=="inet") | .local'
  )" = "10.10.0.1"
""")
