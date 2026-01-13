# Wordlist Filter

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

**Hochperformantes Wordlist-Filtering-Tool für Penetration Testing**

Wordlist Filter ist ein professionelles CLI-Tool zur Verarbeitung sehr großer Wordlists (100-400GB+). Es ermöglicht das Filtern nach Wortlänge, Regex-Patterns und entfernt automatisch Duplikate.

```
╔══════════════════════════════════════════════════════════════════════════════╗
║   ██╗    ██╗ ██████╗ ██████╗ ██████╗ ██╗     ██╗███████╗████████╗           ║
║   ██║    ██║██╔═══██╗██╔══██╗██╔══██╗██║     ██║██╔════╝╚══██╔══╝           ║
║   ██║ █╗ ██║██║   ██║██████╔╝██║  ██║██║     ██║███████╗   ██║              ║
║   ██║███╗██║██║   ██║██╔══██╗██║  ██║██║     ██║╚════██║   ██║              ║
║   ╚███╔███╔╝╚██████╔╝██║  ██║██████╔╝███████╗██║███████║   ██║              ║
║    ╚══╝╚══╝  ╚═════╝ ╚═╝  ╚═╝╚═════╝ ╚══════╝╚═╝╚══════╝   ╚═╝              ║
║   ███████╗██╗██╗  ████████╗███████╗██████╗                                  ║
║   ██╔════╝██║██║  ╚══██╔══╝██╔════╝██╔══██╗                                 ║
║   █████╗  ██║██║     ██║   █████╗  ██████╔╝                                 ║
║   ██╔══╝  ██║██║     ██║   ██╔══╝  ██╔══██╗                                 ║
║   ██║     ██║███████╗██║   ███████╗██║  ██║                                 ║
║   ╚═╝     ╚═╝╚══════╝╚═╝   ╚══════╝╚═╝  ╚═╝                                 ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

## Features

- **Längenfilterung**: Exakte Länge, mehrere Längen oder Bereiche
- **Regex-Pattern**: Komplexe Filterung mit regulären Ausdrücken
- **Automatische Deduplizierung**: Entfernt doppelte Einträge (case-sensitive)
- **Große Dateien**: Optimiert für 100-400GB+ durch Memory-Mapped I/O
- **Encoding-Erkennung**: Automatische Erkennung und Konvertierung verschiedener Zeichensätze
- **Parallele Verarbeitung**: Multi-threaded für maximale Performance
- **Fortschrittsanzeige**: Detaillierte Progress-Bars und Statistiken

## Installation

### Voraussetzungen

- Rust 1.70 oder höher
- Cargo (wird mit Rust installiert)

### Von Source kompilieren

```bash
# Repository klonen
git clone https://github.com/m0h1nd4/wordlist-filter.git
cd wordlist-filter

# Release-Build erstellen
cargo build --release

# Binary befindet sich in:
# ./target/release/wordlist-filter
```

### Schnellinstallation (Linux/macOS)

```bash
git clone https://github.com/m0h1nd4/wordlist-filter.git
cd wordlist-filter
cargo build --release
sudo cp target/release/wordlist-filter /usr/local/bin/
```

### Windows

```powershell
git clone https://github.com/m0h1nd4/wordlist-filter.git
cd wordlist-filter
cargo build --release

# Binary: .\target\release\wordlist-filter.exe
# Optional: Zum PATH hinzufügen
```

## Verwendung

### Grundlegende Syntax

```bash
wordlist-filter -i <INPUT> [OPTIONS]
```

### Längenfilterung

```bash
# Nur Wörter mit exakt 8 Zeichen
wordlist-filter -i wordlist.txt -l 8

# Mehrere Längen (erstellt separate Dateien)
wordlist-filter -i wordlist.txt -l 8,9,10

# Bereich von 8 bis 12 Zeichen
wordlist-filter -i wordlist.txt -l 8-12

# Alle Längen in einer Datei kombinieren
wordlist-filter -i wordlist.txt -l 8-12 --single-file
```

### Regex-Filter

```bash
# Nur Kleinbuchstaben
wordlist-filter -i wordlist.txt -p "^[a-z]+$"

# 4 Buchstaben + 4 Zahlen (z.B. "pass1234")
wordlist-filter -i wordlist.txt -p "^[a-z]{4}[0-9]{4}$"

# Komplexe Passwörter (Groß, Klein, Zahl)
wordlist-filter -i wordlist.txt -p "^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9]).{8,}$"

# Kombination: Länge 8 UND nur Buchstaben
wordlist-filter -i wordlist.txt -l 8 -p "^[a-zA-Z]+$"
```

### Verzeichnisse verarbeiten

```bash
# Einzelnes Verzeichnis (nicht rekursiv)
wordlist-filter -i /path/to/wordlists/ -l 8

# Rekursiv alle Unterverzeichnisse
wordlist-filter -i /path/to/wordlists/ -l 8 --recursive

# Nur bestimmte Dateiendungen
wordlist-filter -i /path/to/wordlists/ -l 8 --extensions "txt,lst,dict"
```

### Ausgabeoptionen

```bash
# Ausgabeverzeichnis festlegen
wordlist-filter -i wordlist.txt -l 8 -o /output/dir/

# Eigener Dateiname für Single-File-Modus
wordlist-filter -i wordlist.txt -l 8-12 --single-file --output-name "combined.txt"
```

### Performance-Optionen

```bash
# Thread-Anzahl festlegen
wordlist-filter -i wordlist.txt -l 8 -t 16

# Größerer Buffer für schnellere I/O
wordlist-filter -i wordlist.txt -l 8 --buffer-size 128MB

# Deduplizierung deaktivieren (schneller, aber Duplikate möglich)
wordlist-filter -i wordlist.txt -l 8 --no-dedup

# Bloom-Filter für sehr große Datenmengen (weniger RAM)
wordlist-filter -i wordlist.txt -l 8 --dedup-strategy bloom
```

### Sonstige Optionen

```bash
# Dry-Run (zeigt was passieren würde)
wordlist-filter -i wordlist.txt -l 8 --dry-run

# Quiet-Modus (minimale Ausgabe)
wordlist-filter -i wordlist.txt -l 8 -q

# Verbose-Modus (detaillierte Informationen)
wordlist-filter -i wordlist.txt -l 8 -v

# Statistiken anzeigen
wordlist-filter -i wordlist.txt -l 8 --stats
```

## Regex-Pattern Referenz

### Grundlegende Pattern

| Pattern | Beschreibung | Beispiel-Match |
|---------|--------------|----------------|
| `^[a-z]+$` | Nur Kleinbuchstaben | password |
| `^[A-Z]+$` | Nur Großbuchstaben | PASSWORD |
| `^[a-zA-Z]+$` | Nur Buchstaben | Password |
| `^[0-9]+$` | Nur Zahlen | 12345678 |
| `^[a-zA-Z0-9]+$` | Alphanumerisch | Pass1234 |

### Struktur-Pattern

| Pattern | Beschreibung | Beispiel-Match |
|---------|--------------|----------------|
| `^[a-z]{4}[0-9]{4}$` | 4 Buchstaben + 4 Zahlen | pass1234 |
| `^[A-Z][a-z]+[0-9]+$` | Großbuchstabe + Klein + Zahlen | Password123 |
| `^[a-z]+[!@#$%]+$` | Buchstaben + Sonderzeichen | password! |
| `.*[!@#$%^&*].*` | Enthält Sonderzeichen | p@ssword |

### Komplexe Pattern

| Pattern | Beschreibung |
|---------|--------------|
| `^(?=.*[a-z])(?=.*[A-Z])(?=.*[0-9]).{8,}$` | Mind. 8 Zeichen, Groß+Klein+Zahl |
| `^(?=.*[!@#$%]).{8,12}$` | 8-12 Zeichen mit Sonderzeichen |
| `^[a-z]{2,4}[0-9]{2,4}[a-z]{2,4}$` | Buchstaben-Zahlen-Buchstaben |

### Regex-Quantifizierer

| Quantifizierer | Bedeutung |
|----------------|-----------|
| `*` | 0 oder mehr |
| `+` | 1 oder mehr |
| `?` | 0 oder 1 |
| `{n}` | Exakt n |
| `{n,}` | Mindestens n |
| `{n,m}` | Zwischen n und m |

## Alle Optionen

```
USAGE:
    wordlist-filter [OPTIONS] -i <PATH>

OPTIONS:
    -i, --input <PATH>              Eingabedatei oder -verzeichnis (erforderlich)
    -o, --output <DIR>              Ausgabeverzeichnis (Standard: aktuelles Verzeichnis)
    -l, --length <LENGTH>           Längenfilter: 8, 8-12, oder 8,9,10
    -p, --pattern <PATTERN>         Regex-Pattern zum Filtern
        --single-file               Alle Ergebnisse in einer Datei
        --output-name <NAME>        Dateiname für Single-File [Standard: filtered_wordlist.txt]
    -r, --recursive                 Verzeichnisse rekursiv verarbeiten
    -t, --threads <NUM>             Thread-Anzahl (Standard: Auto)
        --dedup-strategy <STRAT>    Deduplizierungs-Strategie: memory, bloom [Standard: memory]
        --memory-limit <SIZE>       RAM-Limit für Deduplizierung [Standard: 8GB]
        --no-dedup                  Deduplizierung deaktivieren
        --buffer-size <SIZE>        Buffer-Größe für I/O [Standard: 64MB]
        --extensions <EXT>          Dateiendungen zum Verarbeiten [Standard: txt]
        --stats                     Detaillierte Statistiken anzeigen
    -q, --quiet                     Minimale Ausgabe
    -v, --verbose                   Ausführliche Ausgabe
        --dry-run                   Nur anzeigen, nicht ausführen
        --sort                      Ausgabe alphabetisch sortieren
    -h, --help                      Hilfe anzeigen
    -V, --version                   Version anzeigen
```

## Performance-Empfehlungen

### Für sehr große Dateien (100GB+)

1. **Ausreichend RAM**: Mindestens 16GB für In-Memory-Deduplizierung
2. **SSD empfohlen**: Deutlich schneller als HDD
3. **Bloom-Filter nutzen**: Bei RAM-Knappheit `--dedup-strategy bloom`
4. **Thread-Anzahl**: Entsprechend CPU-Kernen einstellen

### Optimale Einstellungen

```bash
# Für Systeme mit viel RAM (64GB+)
wordlist-filter -i huge_wordlist.txt -l 8 \
    --buffer-size 256MB \
    --memory-limit 32GB \
    -t 16

# Für Systeme mit wenig RAM (8-16GB)
wordlist-filter -i huge_wordlist.txt -l 8 \
    --dedup-strategy bloom \
    --buffer-size 64MB \
    -t 8
```

## Beispiel-Workflow

```bash
# 1. Erstmal prüfen was passieren würde
wordlist-filter -i /wordlists/ -l 8-12 -r --dry-run

# 2. Verarbeitung starten
wordlist-filter -i /wordlists/ -l 8-12 -r -o ./filtered/

# 3. Ergebnisse prüfen
ls -la ./filtered/
wc -l ./filtered/*.txt
```

## Ausgabeformat

### Multi-Length-Modus (Standard)

```
./output/
├── wordlist_len8.txt
├── wordlist_len9.txt
├── wordlist_len10.txt
├── wordlist_len11.txt
└── wordlist_len12.txt
```

### Single-File-Modus

```
./output/
└── filtered_wordlist.txt  (oder eigener Name via --output-name)
```

## Fehlerbehandlung

Das Tool behandelt folgende Situationen automatisch:

- **Ungültige Zeichen**: Werden mit UTF-8 Lossy-Konvertierung behandelt
- **Encoding-Erkennung**: Automatische Erkennung von UTF-8, UTF-16, ISO-8859-1, etc.
- **Leere Zeilen**: Werden automatisch übersprungen
- **Whitespace**: Wird am Anfang und Ende jeder Zeile entfernt

## Lizenz

Dieses Projekt steht unter der Apache License 2.0. Siehe [LICENSE](LICENSE) für Details.

## Autor

- **m0h1nd4** - [GitHub](https://github.com/m0h1nd4)

## Mitwirken

Beiträge sind willkommen! Bitte erstellen Sie einen Pull Request oder öffnen Sie ein Issue.

## Changelog

### v1.0.0
- Initiales Release
- Längenfilterung (einzeln, mehrfach, Bereiche)
- Regex-Pattern-Filter
- Automatische Deduplizierung
- Memory-Mapped I/O für große Dateien
- Automatische Encoding-Erkennung
- Parallele Verarbeitung
- Fortschrittsanzeige mit Statistiken
