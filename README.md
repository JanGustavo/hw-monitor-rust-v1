# HW Monitor · Rust v1

> Monitor de hardware e sistema para Linux, escrito em Rust com GUI nativa via `egui/eframe`.

[![CI](https://github.com/JanGustavo/hw-monitor-rust-v1/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/JanGustavo/hw-monitor-rust-v1/actions/workflows/ci.yml)

---

## Métricas disponíveis

| Card | O que exibe |
|------|-------------|
| **01 / CPU** | Modelo, uso total, uso e clock por thread, estados (user/system/iowait/irq/softirq/steal), context switches/s, interrupções/s |
| **02 / GPU · DRM** | Uso, VRAM usada/total, clock core, driver, vendor/device ID, sensores gráficos |
| **03 / MEMÓRIA & SWAP** | RAM usada/total, disponível, cache, buffers, slab, dirty, swap, page faults/s, major faults/s, pgscan, pgsteal, compact stalls |
| **04 / ARMAZENAMENTO** | Uso e capacidade por filesystem, IOPS de leitura/escrita, throughput, ocupação de fila, latência |
| **05 / REDE** | Taxa RX/TX por interface, pacotes/s, erros, drops, link status, velocidade |
| **06 / SENSORES · HWMON** | Temperatura, tensão, RPM e potência via `/sys/class/hwmon` |
| **07 / SISTEMA & KERNEL** | Hostname, distro, kernel, uptime, load average, processos, TCP retransmits/s, forks/s |
| **08 / ENERGIA & BATERIA** | Status, carga %, potência (W), energia (Wh), tomada |
| **Custo próprio** | CPU %, RSS (MiB) e threads do próprio processo monitor |
| **PSI** | Pressão de CPU, memória e I/O (`avg10`) via `/proc/pressure` |

> **Limitações v1:** Limites de saúde são genéricos (não calibrados por modelo). Temperatura é colorida apenas quando reconhecida como sensor de GPU ou CPU. Clock de GPU requer suporte do driver via DRM sysfs.

---

## Instalação e execução

### Dependências (Pop!_OS 24.04 / Ubuntu / Debian)

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev
```

### Rust

```bash
# Instale via https://rustup.rs/ e reabra o terminal
rustup update stable
```

### Executar

```bash
# Janela nativa (recomendado)
cargo run --release --bin hw-monitor-native

# Servidor web opcional (http://127.0.0.1:7878)
cargo run --release --bin hw-monitor-web
```

### Instalar no menu de aplicativos (ícone no dock)

```bash
bash scripts/install-user.sh
```

O instalador compila a versão release e copia o binário e o ícone para a sua
conta. Depois, abra **HW Monitor** pelo menu de aplicativos. No Wayland, o
`app_id` da janela corresponde ao nome do arquivo `.desktop`; ao executar pelo
`cargo run` sem instalar o lançador, o dock pode mostrar um ícone genérico.
Feche as janelas já abertas e inicie pelo menu para verificar o ícone. A imagem
no cabeçalho e o ícone da janela são carregados do mesmo desenho, em 256 px.

> Os dois binários são processos separados e independentes. Cada um roda seu próprio coletor; não há comunicação entre eles.

---

## Medição de consumo

Para medir o custo real do monitor, aguarde a janela estabilizar (~30 s) e observe o card **"CUSTO DO PRÓPRIO MONITOR"** na parte inferior do painel. Ou via terminal:

```bash
ps -p $(pgrep -f hw-monitor-native) -o pid,%cpu,rss,nlwp,comm
```

> **Nota:** Os números de consumo serão publicados após medição em hardware específico (CPU, RAM e condições do teste serão informadas). Não assumir baixo consumo sem medir.

---

## Organização do código

```
src/
├── lib.rs            # Expõe collector e health como biblioteca compartilhada
├── main.rs           # Inicialização da janela nativa (App + eframe)
├── collector.rs      # Loop de coleta Linux (/proc, /sys, hwmon, DRM)
├── health.rs         # Paleta de cores e regras de classificação de saúde
├── self_monitor.rs   # Medição do próprio processo (/proc/self/stat|status)
└── ui/
    └── cards.rs      # Cards, gauge, row, dashboard, header (toda a UI)
src/bin/
└── web.rs            # Servidor HTTP + WebSocket (reutiliza collector via lib)
static/
├── logohw.png        # Arte original
├── icon-256.png      # Ícone da janela e do cabeçalho
└── index.html        # Frontend web (servido pelo hw-monitor-web)
```

---

## Tecnologias

- **[Rust](https://www.rust-lang.org/)** — linguagem principal
- **[eframe / egui](https://github.com/emilk/egui)** `0.36.2` — GUI nativa com renderização wgpu
- **[axum](https://github.com/tokio-rs/axum)** `0.6` — servidor HTTP/WebSocket para o modo web
- **[tokio](https://tokio.rs/)** — runtime assíncrono para o coletor e servidor web
- Fontes de dados: `/proc`, `/sys`, `/sys/class/hwmon`, `/sys/class/drm`, `/sys/class/power_supply`

---

## Licença

[MIT](LICENSE)
