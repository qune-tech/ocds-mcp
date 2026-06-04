/// Reference guide for the German procurement data this server exposes.
///
/// Served as a resource (`vergabe://guide`) so LLMs can interpret the data
/// they fetch. Bilingual (German-first with English context) for optimal
/// use by German LLMs.
pub const VERGABE_GUIDE: &str = r#"# Vergabedaten — Referenzhandbuch / Reference Guide

## Überblick / Overview

Dieser MCP-Server ist ein **lokaler Thin Client** für die `Vergabe Dashboard API`.
Suchanfragen und Firmenbeschreibungen werden **lokal** per multilingual-e5-small
(384-dim) eingebettet; nur der resultierende Vektor, OCIDs und Filterwerte
verlassen die Maschine — **niemals der Anfrage- oder Profiltext**.

Die Daten stammen aus **eForms-XML** (UBL) von oeffentlichevergabe.de, dem
deutschen Vergabeportal. Alle Texte (Titel, Beschreibungen, Organisationsnamen)
sind auf **Deutsch**. Suchanfragen und Firmenbeschreibungen sollten daher
ebenfalls auf Deutsch formuliert werden, um die beste Matching-Qualität zu
erzielen.

---

## Datenschutz-Architektur / Privacy architecture

```
LLM ←stdio→ vergabe-mcp (dieser Server, lokal)
               │  lokal: Firmenprofile (SQLite) + SentenceEmbedder (ONNX)
               │  HTTP: Vektoren, OCIDs, Filterwerte, API-Schlüssel
               └──HTTPS──→ Vergabe Dashboard API (/api/v1)
```

- **search_text** bettet die Anfrage lokal ein (Präfix `query: `) und sendet
  nur den 384-Float-Vektor an `POST /api/v1/search/vector`.
- **match_tenders** sendet den lokal gespeicherten Profilvektor (Präfix
  `passage: `, lokal berechnet) an `POST /api/v1/match/vector`.
- Firmenprofile (Name, Beschreibung, CPV-Interessen, Standort) liegen
  ausschließlich in einer lokalen SQLite-Datenbank.
- Was die API sieht: Vektoren, abgerufene OCIDs, Filterwerte und den
  API-Schlüssel — genug, um *kommerzielles Interesse* abzuleiten, aber nicht
  den Profil- oder Anfragetext selbst.

Beim ersten Start lädt der Server das Embedding-Modell (model.onnx +
tokenizer.json, ~118 MB) von **huggingface.co** und legt es unter
`~/.cache/vergabe/models/multilingual-e5-small` ab. Danach ist kein
HuggingFace-Kontakt mehr nötig.

---

## Empfohlene Workflows / Recommended Workflows

### Ersteinrichtung

1. **`get_index_info`** — Konnektivität, API-Status/-Version, Embedder-Status
   und den Vergleich des Embedding-Vertrags (Client vs. Server) prüfen.
2. Dieses Handbuch (**`vergabe://guide`**) lesen.

### Ausschreibungen finden

| Ziel | Tool | Hinweise |
|------|------|----------|
| Semantische Textsuche | `search_text` | Deutsche Anfrage, z.B. "IT-Sicherheit öffentliche Verwaltung". Lokal eingebettet, Vektorsuche serverseitig. |
| Strukturierte Filterung | `list_releases` | Nur Filter, keine Semantik. Paginiert (limit/offset). |
| eForms-XML einer Ausschreibung | `get_release` | OCID übergeben. Liefert die dünne Hülle inkl. `raw_xml` — das XML selbst nach Details parsen. |
| Verfahrensverlauf (Geschwister) | `linked_notices` | OCID übergeben, erhält die `{ocid, notice_id}`-Liste des Verfahrens. |

**Typischer Ablauf:** `search_text` / `list_releases` → interessante OCIDs →
`get_release` für das eForms-XML → ggf. `linked_notices`, um Geschwister
(PIN/CN/CAN) zu finden und einzeln per `get_release(notice_id=…)` abzurufen.

**Beispiel: "Finde IT-Ausschreibungen für Cloud-Services"**
```
1. search_text(query="Cloud-Infrastruktur Managed Services Rechenzentrum",
               filters={"cpv_starts_with":"72","phase":"open"})
2. → SearchResult-Zeilen mit ocid, score, title, buyer_name, deadline, phase …
3. get_release(ocid="ocds-mnwr74-…") → raw_xml parsen
```

### Firmenprofil-Matching / Ausschreibungen für mein Unternehmen finden

1. **`create_company_profile`** — Name, Beschreibung (auf Deutsch!), optional
   CPV-Codes, Kategorien, Standort. Die Beschreibung wird **lokal** eingebettet.
   Je konkreter die Beschreibung (Fachgebiete, Technologien, Projektgrößen),
   desto besser das Matching.
2. **`match_tenders`** — findet Ausschreibungen, die semantisch zum Profil
   passen. Optionale `filters` wirken serverseitig.
3. Details mit `get_release` abrufen.

**Beispiel: "Welche Ausschreibungen passen zu meiner IT-Firma?"**
```
1. create_company_profile(
     name="MeineFirma GmbH",
     description="IT-Dienstleister für die öffentliche Verwaltung.
       Softwareentwicklung, Cloud-Infrastruktur, IT-Sicherheit. Erfahrung mit
       E-Government, OZG-Umsetzung und digitaler Transformation.",
     cpv_codes=["72000000"], categories=["services"], location="Berlin, Germany")
2. match_tenders(profile_id="…", k=10, filters={"phase":"open"})
3. get_release(ocid="…") für die Top-Treffer
```

Profilverwaltung: `get_company_profile`, `list_company_profiles`,
`update_company_profile`, `delete_company_profile`.

### Verfahrensverlauf / Lifecycle walk

Ein Vergabeverfahren besteht oft aus mehreren Bekanntmachungen
(Vorinformation → Ausschreibung → Zuschlag). Die Zugehörigkeit ist das eine,
was ein Client nicht selbst ableiten kann:

```
1. linked_notices(ocid="…")
   → {key, notices:[{ocid, notice_id}, …]}   (in created_at-Reihenfolge)
2. Für jede notice_id: get_release(ocid="…", notice_id="…")
   → raw_xml der jeweiligen Bekanntmachung parsen
```

Eine Liste der Länge 1 bedeutet: ein einzelnes Verfahren ohne weitere
Geschwister (typisch für unterschwellige Vergaben).

---

## Tool-Referenz (11 Tools)

### Suche & Recherche
| Tool | Beschreibung |
|------|-------------|
| `search_text` | Semantische Suche: Anfrage wird lokal eingebettet, nur der Vektor + Filter gehen an die API. Liefert SearchResult-Zeilen. |
| `list_releases` | Reine Filter-Abfrage (keine Semantik). Paginiert. Liefert SearchResult-Zeilen, nach deadline DESC. |
| `get_release` | eForms-XML-Hülle zu einer OCID: `{ocid, notice_id, data_source, country, raw_xml}`. Optional `notice_id` für ein bestimmtes Geschwister. |
| `linked_notices` | Verfahrenszugehörigkeit: `{key, notices:[{ocid, notice_id}]}`. Nur IDs. |

### Firmenprofile & Matching
| Tool | Beschreibung |
|------|-------------|
| `create_company_profile` | Profil lokal speichern, Beschreibung lokal eingebettet. Gibt UUID + Embedding-Status. |
| `update_company_profile` | Teilaktualisierung; bei Beschreibungsänderung neu eingebettet. |
| `get_company_profile` | Vollständiges Profil nach UUID. |
| `list_company_profiles` | Alle Profile (Zusammenfassung), nach Erstellungsdatum. |
| `delete_company_profile` | Profil und Embedding löschen. |
| `match_tenders` | Semantisches Matching gegen den lokal gespeicherten Profilvektor. Liefert SearchResult-Zeilen. |

### Serverstatus
| Tool | Beschreibung |
|------|-------------|
| `get_index_info` | API-Status/-Version, Embedder-Status, lokale Profilanzahl, Vergleich des Embedding-Vertrags. Zuerst aufrufen! |

---

## SearchResult — Zeilenformat / row shape

`search_text`, `list_releases` und `match_tenders` liefern alle dieselbe
SearchResult-Zeile:

| Feld | Bedeutung |
|------|-----------|
| `ocid` | Open Contracting ID — Primärschlüssel über Bekanntmachungen hinweg. |
| `score` | Kosinus-Ähnlichkeit (nur bei semantischer Suche/Match). |
| `title` | Titel der Ausschreibung (Deutsch). |
| `buyer_name` | Name des Auftraggebers (befüllt, auch unterschwellig). |
| `procurement_method` | Roher eForms-Verfahrenscode (z.B. `de-open`, `neg-w-call`). |
| `main_procurement_category` | `goods`, `works` oder `services`. |
| `value_amount` / `value_currency` | Geschätzter Wert + Währung. |
| `currency_eur_amount` | In EUR umgerechneter Wert (für Wertfilter). |
| `deadline` | Abgabefrist als RFC3339 (aus dem Epoch-Sekunden-Index abgeleitet). |
| `award_date` | Zuschlagsdatum (bei vergebenen Verfahren). |
| `cpv_codes` | CPV-Codes der Ausschreibung. |
| `phase` | Lebenszyklusphase (siehe unten). |
| `documents_url` | Link zu den Vergabeunterlagen (wenn vorhanden). |
| `data_source` | Datenquelle (z.B. `de`). |
| `country` | Land (ISO-2). |

Für den vollständigen eForms-Inhalt: `get_release(ocid)` und das `raw_xml` parsen.

---

## Lebenszyklusphasen / Lifecycle phases

Das Feld `phase` ist eine abgeleitete, einwertige Lebenszyklusphase (kein
OCDS-`tag`-Array). Filterbare und vorkommende Werte:

| Phase | Bedeutung |
|-------|-----------|
| `planning` | Vorinformation (PIN) — noch keine Angebote möglich. |
| `open` | Laufende Ausschreibung (CN) — Frist noch nicht abgelaufen. |
| `closed` | Frist abgelaufen, noch kein Zuschlag veröffentlicht. |
| `awarded` | Zuschlag erteilt (CAN). |
| `unsuccessful` | Verfahren ohne Zuschlag beendet. |

Filtere mit `phase` in `search_text`, `list_releases`, `match_tenders`.

---

## Filter-Taxonomie / Filter taxonomy

Dieselben Feldnamen in allen drei gefilterten Tools. `search_text` und
`match_tenders` senden sie als JSON-Objekt `filters`; `list_releases`
übersetzt sie in Query-Parameter (`procurement_method` als wiederholten
Parameter).

| Filter | Bedeutung |
|--------|-----------|
| `phase` | Lebenszyklusphase (planning/open/closed/awarded/unsuccessful). |
| `cpv_starts_with` | CPV-Präfix, 2–8 Stellen (z.B. `45` Bau, `72` IT-Dienstleistungen, `33` Medizintechnik). |
| `country` | ISO-3166 alpha-2 (z.B. `DE`). |
| `value_min` / `value_max` | Geschätzter Wert in EUR. |
| `deadline_after` / `deadline_before` | RFC3339-Datum; gegen den Frist-Index gematcht. |
| `procurement_method` | Liste roher eForms-Verfahrenscodes (match-any), z.B. `["de-open","neg-w-call"]`. |
| `main_procurement_category` | `goods`, `works` oder `services`. |
| `buyer_name` | Auftraggebername, exakter Abgleich (case-insensitiv). |
| `data_source` | Datenquelle (z.B. `de`). |

Hinweis: Es gibt **keinen** `month`-Filter. Für Zeitfenster
`deadline_after` / `deadline_before` verwenden.

---

## Datenquelle und eForms / Data source

Die Daten stammen aus eForms-XML (UBL) von oeffentlichevergabe.de. `get_release`
liefert das rohe XML in `raw_xml`; daraus lassen sich u.a. ableiten:

- **Fristen** — Abgabe-/Angebotsfristen (auch in der SearchResult-Zeile als
  `deadline`, RFC3339, vorhanden).
- **Eignungskriterien** (`selectionCriteria`) — Typen wie `sui-act`
  (Befähigung), `tp-abil` (technische Leistungsfähigkeit), `ef-stand`
  (wirtschaftliche Leistungsfähigkeit).
- **Zuschlagskriterien** (`awardCriteria`) — `price`, `quality`, `cost` mit
  Gewichtung.
- **Lose** (`lots`) — eigene Werte, Beschreibungen, oft eigene
  Mindestanforderungen.
- **Beteiligte** (Auftraggeber, Bieter, Zuschlagsempfänger). Hinweis:
  Zuschlagsempfänger sind in deutschen Daten häufig nur über die Rolle
  `tenderer`/`supplier` in den Beteiligten zu finden; das XML kann
  Vertraulichkeitsflags (`FieldsPrivacy`) enthalten.

Wegen der Vielfalt der eForms-Felder gilt: **das XML selbst parsen** statt sich
auf eine feste JSON-Projektion zu verlassen.

---

## Deutsches Vergaberecht — Kurzüberblick

Hilft, Ausschreibungen einzuordnen und Nutzer bei der Bewerbungsvorbereitung
zu unterstützen.

### Rechtsrahmen

| Ebene | Gesetz/Verordnung | Anwendung |
|-------|-------------------|-----------|
| EU-Schwellenwerte | GWB Teil 4 + VgV | Liefer-/Dienstleistungen ab 221.000 EUR, Bauleistungen ab 5.538.000 EUR |
| Unter Schwellenwert | UVgO (Liefer/DL), VOB/A (Bau) | Nationale Vergabe, weniger formale Anforderungen |

### Vergabeverfahren (procurement_method)

Das Feld trägt rohe eForms-Verfahrenscodes. Häufige Werte und ihre Bedeutung:

| eForms-Code (Beispiele) | Verfahren | Bedeutung |
|-------------------------|-----------|-----------|
| `de-open`, `open` | Offenes Verfahren | Jedes Unternehmen kann ein Angebot abgeben. Höchster Wettbewerb. |
| `restricted`, `selective` | Nichtoffenes Verfahren | Erst Teilnahmewettbewerb (Eignung), dann Angebotsaufforderung. |
| `neg-w-call` | Verhandlungsverfahren mit Teilnahmewettbewerb | Verhandlungen nach Teilnahmewettbewerb. |
| `neg-wo-call` | Verhandlungsverfahren ohne Teilnahmewettbewerb | Einladung ausgewählter Unternehmen, nur in Ausnahmen. |

Filtere mit `procurement_method=["de-open", …]` (match-any). Die genaue
Codeliste variiert; bei Unsicherheit das `raw_xml` prüfen.

### Typische Eignungsanforderungen

- **Eigenerklärungen** — keine Ausschlussgründe (§§ 123, 124 GWB), Mindestlohn
  (MiLoG), Russland-Sanktionen (EU-VO 833/2014, Art. 5k).
- **Wirtschaftliche Leistungsfähigkeit** (§ 45 VgV) — Jahresumsatz,
  Betriebshaftpflicht.
- **Technische Leistungsfähigkeit** (§ 46 VgV) — Referenzprojekte,
  Qualifikationsprofile, ggf. ISO 9001 / ISO 27001.
- **Technisches Konzept** — Lösungsansatz, Personalkonzept,
  Qualitätssicherung.

### Formvorschriften & häufige Fehler

- Elektronische Abgabe ist Pflicht; Textform genügt meist.
- **Angebotsfrist ist absolut** — eine Sekunde zu spät = zwingender Ausschluss.
- Häufige Fehler: Fristversäumnis, fehlende Preisangaben, eigene AGB beigefügt
  (= Änderung der Vergabeunterlagen = Ausschluss), nicht vergleichbare
  Referenzen.

### Präqualifikation (AVPQ)

Das Amtliche Verzeichnis Präqualifizierter Unternehmen (AVPQ) bei den IHKs gilt
als vorläufiger Eignungsnachweis und beschleunigt Bewerbungen.

---

## CPV-Codes (Gemeinsames Vokabular für öffentliche Aufträge)

CPV-Codes klassifizieren den Beschaffungsgegenstand. Nutze Präfixe
(`cpv_starts_with`) für branchenweite Filterung.

```
XX______  Abteilung  (2 Stellen) — breiteste Kategorie
XXXX____  Gruppe     (4 Stellen)
XXXXXXXX  Kategorie  (8 Stellen) — spezifischste Ebene
```

### Wichtige Abteilungen

| Code | Abteilung (DE) | Division (EN) |
|------|----------------|---------------|
| 30–32 | Büro-/IT-Ausstattung, Elektronik, Telekommunikation | Office/IT equipment, electronics, telecom |
| **33** | **Medizinische Geräte, Pharmazeutika** | Medical equipment, pharmaceuticals |
| **45** | **Bauarbeiten** | Construction work |
| **48** | **Softwarepakete, IT-Systeme** | Software packages, IT systems |
| 50–51 | Reparatur/Wartung, Installation | Repair/maintenance, installation |
| 70–71 | Immobilien, Architektur, Ingenieurwesen | Real estate, architecture, engineering |
| **72** | **IT-Dienstleistungen, Beratung, Softwareentwicklung** | IT services, consulting, software development |
| 73 | F&E-Dienstleistungen | R&D services |
| **79** | **Unternehmensdienstleistungen (Recht, Buchhaltung, Beratung)** | Business services (legal, accounting, consulting) |
| 80 | Bildung, Ausbildung | Education, training |
| 85 | Gesundheit, Sozialwesen | Health, social work |
| **90** | **Abwasser, Abfall, Reinigung, Umwelt** | Sewage, refuse, cleaning, environmental |

**Tipp:** `cpv_starts_with="45"` für Bau, `"72"` für IT-Dienstleistungen,
`"33"` für Medizintechnik.

---

## Praktische Tipps

1. **Immer auf Deutsch suchen** — Titel und Beschreibungen sind deutsch;
   deutsche Suchbegriffe und Firmenbeschreibungen liefern bessere Treffer.
2. **OCID ist der Primärschlüssel** — dasselbe Verfahren erscheint über mehrere
   Bekanntmachungen hinweg unter Geschwister-Beziehungen; `linked_notices`
   liefert die Zugehörigkeit.
3. **Phasen statt Tags** — `phase` ist einwertig (planning/open/closed/
   awarded/unsuccessful). Offene, bewerbbare Verfahren: `phase="open"`.
4. **Wertfilter** — `value_min`/`value_max` arbeiten gegen den EUR-Wert
   (`currency_eur_amount`).
5. **Fristen** — `deadline_after`/`deadline_before` (RFC3339). Kein `month`-Filter.
6. **Geschwister abrufen** — über `linked_notices` die `notice_id` holen und
   `get_release(ocid, notice_id)` aufrufen, um ältere Bekanntmachungen
   desselben Verfahrens zu lesen.
7. **Details aus dem XML** — `get_release` liefert die dünne Hülle; alle
   eForms-Detailfelder (Kriterien, Lose, Beteiligte) aus `raw_xml` parsen.
8. **Privatsphäre** — Anfrage- und Profiltext bleiben lokal; nur Vektoren,
   OCIDs und Filterwerte gehen an die API. Bei der ersten Nutzung wird das
   Embedding-Modell von huggingface.co geladen (einmalig, danach gecacht).
9. **Bewerbungsvorbereitung** — Dieser Server findet und bewertet
   Ausschreibungen; die eigentliche Angebotserstellung kann das LLM direkt auf
   Basis der Ausschreibungsdaten unterstützen.
"#;
