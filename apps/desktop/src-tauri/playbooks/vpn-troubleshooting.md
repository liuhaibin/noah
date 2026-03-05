---
name: vpn-troubleshooting
description: Diagnose VPN connection failures, drops, and split-tunnel DNS issues
platform: macos
---

# VPN Troubleshooting

## When to activate
User reports: VPN won't connect, VPN keeps disconnecting, can't access work resources on VPN, slow internet with VPN, DNS not resolving with VPN on.

## Quick check
Run `mac_network_info` — look for a `utun` or `ipsec` interface (VPN tunnel).
- If VPN interface exists with an IP → tunnel is up. Problem is likely DNS or routing. Jump to step 3.
- If no VPN interface → VPN is not connected. Start at step 1.

## Standard fix path (try in order)

### 1. Verify internet connectivity
Run `mac_ping` to `8.8.8.8`. VPN requires a working internet connection first.
- If ping fails → fix the internet connection first. Activate `network-diagnostics` playbook.
- If ping works → internet is fine. Continue.

### 2. Reconnect VPN
Disconnect and reconnect the VPN client. Many transient failures resolve on reconnect.
- If the VPN client shows an error → note the error message for diagnosis.
- If it connects but immediately drops → check for conflicting VPN clients. Only one should be active at a time.
- If it won't connect at all → check: has the password changed? Does MFA (Duo, Okta) need to be completed? Is the VPN client up to date?

### 3. Fix DNS (the #1 VPN issue)
Most "VPN connected but can't access anything" problems are DNS.
Run `mac_dns_check` for an internal hostname (e.g., `intranet.company.com`) AND an external one (`google.com`).

- **Internal fails, external works** → VPN DNS server isn't in the resolver chain. Fix: `mac_flush_dns` to clear stale cache, then reconnect VPN.
- **Both fail** → VPN is capturing all DNS but its DNS server is unreachable. Disconnect VPN, verify DNS works, reconnect.
- **Both work** → DNS is fine. The problem is routing — specific subnets may not route through the VPN tunnel.

### 4. Flush DNS and reconnect
Run `mac_flush_dns`, then disconnect and reconnect the VPN.
After connecting/disconnecting VPN multiple times, macOS DNS resolver can get confused. A clean flush + reconnect resets the state.

> Steps 1-4 resolve ~80% of VPN issues. The #1 cause: DNS misconfiguration after VPN connect/disconnect cycles.

## Caveats
- **Port blocking:** Hotel/airport/corporate-guest Wi-Fi often blocks VPN ports (500, 4500 for IKEv2; 1194 for OpenVPN). If the VPN has an SSL/HTTPS mode (port 443), suggest trying that — it's rarely blocked.
- **macOS sleep disconnects VPN** by default. Some VPN clients have a "reconnect after wake" setting. If VPN drops every time the Mac sleeps, this is normal behavior, not a bug.
- **Full-tunnel vs split-tunnel:** If internet is slow with VPN on, the VPN may be routing ALL traffic (full tunnel). This is expected — the VPN server's connection is the bottleneck. Ask IT if split-tunnel is available.

## Key signals
- **"VPN works from home but not this hotel"** → port blocking. Try SSL mode or a mobile hotspot as a workaround.
- **"Can't access [internal site] but internet works fine"** → DNS issue. Step 3.
- **"VPN worked until macOS update"** → check if VPN system extension is still allowed. System Settings → Privacy & Security → Network Extensions.
- **"VPN says 'HIP check failed'" (GlobalProtect)** → the client is checking system compliance (OS version, antivirus, FileVault). Update macOS and ensure FileVault is enabled.
- **"Keeps asking for credentials"** → clear Keychain entries for the VPN client and re-enter credentials.

## Tools referenced
- `mac_ping` — basic connectivity test
- `mac_network_info` — check for VPN tunnel interface
- `mac_dns_check` — test internal vs external DNS
- `mac_flush_dns` — clear DNS cache
- `mac_process_list` — check if VPN client process is running
- `mac_http_check` — test HTTP access through VPN

## Escalation
If VPN issues can't be resolved locally:
- Contact IT/help desk with: VPN client name and version, error message, and whether it worked before.
- Many VPN issues are server-side (expired certificates, policy changes, server maintenance).
