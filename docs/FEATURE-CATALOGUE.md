# Feature Catalogue — Vision Index & Release Mapping

This catalogue indexes the 23 vision sections from the original Net Watch Pro vision document, mapped to their target releases per `docs/ROADMAP.md` (2026-11-23).

| # | Vision Section | Target Release | Notes |
|---|---|---|---|
| 1 | Deep Application Intelligence | v0.11, v1.1 | v0.11: profile page, sessions as cgroup lifetime; v1.1: idle consumption, classification, fingerprint history. Foreground/background mapped to active/idle/locked. |
| 2 | Application Behavior Score | v0.11, v1.1 | v0.11: baseline metrics; v1.1: change detection, explainable score (weighted sum, no risk wording). |
| 3 | Anomaly Detection and Alerts | v1.1 | Threshold alerts ship in v0.10 notifications; statistical baselines ship in v1.1. |
| 4 | Network Forensic Explorer | v0.11, v1.2 | v0.11: IPv4 connections (IPv6 if spike passes); v1.2: hostnames via DNS observation. |
| 5 | Connection Timeline and Investigation | v1.2, v1.4 | v1.2: DNS correlation; v1.4: unified timeline and investigation view. |
| 6 | Domain and Destination Intelligence | v1.2 | Observed DNS only; DoH/DoT labelled unknown; optional offline geo lookup. |
| 7 | Forecasting and Usage Planning | v0.10 | Month-end/weekly forecast, budget exhaustion, FUP tracking; per-app forecasting in v1.1. |
| 8 | Smart Budget System | v0.10 | Quotas (total, upload, download), per-app and per-interface budgets, thresholds, daily allowance. |
| 9 | Network Performance Intelligence | v1.3 | Latency, jitter, loss, gateway health, DNS speed, Wi-Fi signal, link speed, error counters. |
| 10 | Network Profiles and Context | v0.11, v1.3 | v0.11: network identity, trust level; v1.3: full profiles with per-profile budgets and policies. |
| 11 | Interface and System Integration | v0.10, v1.5 | v0.10: StatusNotifier tray, compact mode, notifications; v1.5: desktop adapters (layer-shell widgets). |
| 12 | Advanced Traffic Controls | v2.0 (only with owner approval) | Separate privileged component (`netwatch-guard`), nftables cgroup matching; excluded from v1.0. |
| 13 | Reporting, Automation, Export | v0.10, v0.11, v1.4 | v0.10: CSV, JSON, aggregated historical export; v0.11: backup/restore; v1.4: PDF, scheduled reports. |
| 14 | Network Intelligence Engine | v1.1 | Shared baseline engine serving sections 2, 3, 19, and 21. |
| 15 | Network Time Machine | v1.4 | Time-based browsing, historical playback, state comparison. |
| 16 | Traffic Comparison Studio | v0.11, v1.3, v1.4 | v0.11: apps, periods, interfaces, directions; v1.3: profiles; v1.4: multi-entity. |
| 17 | Usage Heatmap | v0.10 | Daily calendar, hourly distribution, weekday patterns from rollups. |
| 18 | Usage Leaderboard | v0.10, v1.2 | v0.10: top apps by total, upload, download, peak; v1.2: top destinations/domains. |
| 19 | Idle Activity Investigator | v1.1 | Consumed data during user idle/lock states (logind D-Bus IdleHint/LockedHint). |
| 20 | Event Correlation Engine | v1.4 | Multi-source event correlation (app launches, network switches, traffic bursts). |
| 21 | Application Traffic Fingerprinting | v1.1 | Behavioral metrics (volume, timing, direction ratio); shown as metrics, not heuristic security labels. |
| 22 | Smart History Archive | v0.9.3, v0.11 | v0.9.3: integrity verification check; v0.11: retention policies, tiered downsampling, archive size view. |
| 23 | Additional Ultimate-Level Features | v0.9.3–v0.10, v0.11, v1.4, v2.0 | v0.9.3: health diagnostics; v0.10: CLI (`nwp`); v0.11: backup; v1.4: bookmarks/tags; v2.0: custom dashboards/plugins. |

---

## 1. Deep Application Intelligence

**Target release:** v0.11, v1.1  
*Full text pending from owner.*

---

## 2. Application Behavior Score

**Target release:** v0.11, v1.1  
*Full text pending from owner.*

---

## 3. Anomaly Detection and Alerts

**Target release:** v1.1  
*Full text pending from owner.*

---

## 4. Network Forensic Explorer

**Target release:** v0.11, v1.2  
*Full text pending from owner.*

---

## 5. Connection Timeline and Investigation

**Target release:** v1.2, v1.4  
*Full text pending from owner.*

---

## 6. Domain and Destination Intelligence

**Target release:** v1.2  
*Full text pending from owner.*

---

## 7. Forecasting and Usage Planning

**Target release:** v0.10  
*Full text pending from owner.*

---

## 8. Smart Budget System

**Target release:** v0.10  
*Full text pending from owner.*

---

## 9. Network Performance Intelligence

**Target release:** v1.3  
*Full text pending from owner.*

---

## 10. Network Profiles and Context

**Target release:** v0.11, v1.3  
*Full text pending from owner.*

---

## 11. Interface and System Integration

**Target release:** v0.10, v1.5  
*Full text pending from owner.*

---

## 12. Advanced Traffic Controls

**Target release:** v2.0 (only with owner approval)  
*Full text pending from owner.*

---

## 13. Reporting, Automation, Export

**Target release:** v0.10, v0.11, v1.4  
*Full text pending from owner.*

---

## 14. Network Intelligence Engine

**Target release:** v1.1  
*Full text pending from owner.*

---

## 15. Network Time Machine

**Target release:** v1.4  
*Full text pending from owner.*

---

## 16. Traffic Comparison Studio

**Target release:** v0.11, v1.3, v1.4  
*Full text pending from owner.*

---

## 17. Usage Heatmap

**Target release:** v0.10  
*Full text pending from owner.*

---

## 18. Usage Leaderboard

**Target release:** v0.10, v1.2  
*Full text pending from owner.*

---

## 19. Idle Activity Investigator

**Target release:** v1.1  
*Full text pending from owner.*

---

## 20. Event Correlation Engine

**Target release:** v1.4  
*Full text pending from owner.*

---

## 21. Application Traffic Fingerprinting

**Target release:** v1.1  
*Full text pending from owner.*

---

## 22. Smart History Archive

**Target release:** v0.9.3, v0.11  
*Full text pending from owner.*

---

## 23. Additional Ultimate-Level Features

**Target release:** v0.9.3–v0.10, v0.11, v1.4, v2.0  
*Full text pending from owner.*
