/// Comprehensive reference guide for interpreting OCDS procurement data.
///
/// Served as a resource via MCP so LLMs can understand the data they fetch.
/// Bilingual (German-first with English context) for optimal use by German LLMs.
pub const OCDS_GUIDE: &str = r#"# OCDS Vergabedaten — Referenzhandbuch / Reference Guide

## Überblick / Overview

Dieser MCP-Server ist ein **Thin Client**, der sich mit der `Vergabe Dashboard API` REST-API
für Ausschreibungsdaten und Vektorsuche verbindet. Firmenprofile werden lokal verwaltet
und automatisch per multilingual-e5-small (384-dim) eingebettet.

Die Daten folgen dem Open Contracting Data Standard (OCDS) von oeffentlichevergabe.de,
dem deutschen Vergabeportal. Alle Texte (Titel, Beschreibungen, Organisationsnamen)
sind auf **Deutsch**. Suchanfragen und Firmenbeschreibungen sollten daher ebenfalls
auf Deutsch formuliert werden, um die beste Matching-Qualität zu erzielen.

---

## Architektur

```
LLM ←stdio→ ocds-mcp (dieser Server)
               │  lokal: Firmenprofile + SentenceEmbedder
               │  remote: Suche, Ausschreibungsdaten
               └──HTTP──→ Vergabe Dashboard API (REST-API, verwaltet Ausschreibungen + Embeddings)
```

Die REST-API muss laufen, damit Suche, Ausschreibungsabruf und Listenabfragen funktionieren.
Firmenprofil-Verwaltung und Embedding funktionieren auch ohne REST-API.

---

## Empfohlene Workflows / Recommended Workflows

### Ersteinrichtung

1. **`get_index_info`** — Konnektivität und Datenbankstatus prüfen: Anzahl
   Ausschreibungen, Embeddings, Embedder-Status.
2. Dieses Handbuch (**`ocds://guide`**) lesen, um das Datenmodell zu verstehen.

### Ausschreibungen finden

| Ziel | Tool | Hinweise |
|------|------|----------|
| Semantische Textsuche | `search_text` | Deutsche Suchanfrage eingeben, z.B. "IT-Sicherheit öffentliche Verwaltung". Wird lokal eingebettet und per KNN-Kosinus-Suche gegen Ausschreibungs-Chunks gematcht. |
| Strukturierte Filterung (Monat, CPV, Kategorie, Wert, Auftraggeber) | `list_releases` | SQL-basiert. Unterstützt Paginierung (limit/offset). Gut zum Durchstöbern nach harten Kriterien. |
| Vollständige Details einer Ausschreibung | `get_release` | OCID aus Such- oder Listenergebnissen übergeben. Liefert den kompletten Release als JSON. |

**Typischer Ablauf:** `search_text` oder `list_releases` → interessante OCIDs auswählen → `get_release` für Details.

**Beispiel: "Finde IT-Ausschreibungen für Cloud-Services"**
```
1. search_text(query="Cloud-Infrastruktur Managed Services Rechenzentrum")
2. → Ergebnisse mit OCIDs, Scores, Chunk-Texten
3. get_release(ocid="ocds-mnwr74-...") für die besten Treffer
```

### Firmenprofil-Matching / Ausschreibungen für mein Unternehmen finden

1. **`create_company_profile`** — Name, Beschreibung (auf Deutsch!), optional
   CPV-Codes, Kategorien und Standort. Die Beschreibung wird automatisch eingebettet.

   **Tipp:** Je konkreter die Beschreibung, desto besser das Matching. Nennen Sie
   Fachgebiete, Technologien, Branchenerfahrung und typische Projektgrößen.

2. **`match_tenders`** — Findet Ausschreibungen, die semantisch zum Firmenprofil
   passen. Ergebnisse werden mit Release-Metadaten angereichert. Unterstützt
   Nachfilter: CPV-Präfix, Kategorie, Vergabeverfahren, Wertbereich, Auftraggeber,
   Frist, Status.

3. Details mit `get_release` abrufen.

**Beispiel: "Welche Ausschreibungen passen zu meiner IT-Firma?"**
```
1. create_company_profile(
     name="MeineFirma GmbH",
     description="IT-Dienstleister für die öffentliche Verwaltung. Softwareentwicklung,
       Cloud-Infrastruktur, IT-Sicherheit. Erfahrung mit E-Government, OZG-Umsetzung
       und digitaler Transformation.",
     cpv_codes=["72000000"],
     categories=["services"],
     location="Berlin, Germany"
   )
2. match_tenders(profile_id="...", k=10)
3. get_release(ocid="...") für die Top-Treffer
```

Profilverwaltung: `get_company_profile`, `list_company_profiles`,
`update_company_profile`, `delete_company_profile`.

### Wettbewerbsanalyse / Konkurrenten identifizieren

Aus vergebenen Ausschreibungen lassen sich Mitbewerber ableiten:

1. **`match_tenders`** mit `tag="award"` und `has_awards=true` — filtert direkt
   auf Zuschlagsveröffentlichungen mit Lieferanteninformationen.
2. **`get_release`** — Für jeden Treffer die vollständigen Daten abrufen.
3. **Auswerten:** In `parties` nach Rollen `tenderer` oder `supplier` suchen.

**Wichtiger Hinweis zur Datenqualität:** Das Feld `awards[].suppliers` ist in den
deutschen OCDS-Daten häufig leer! Die Gewinner sind oft nur über die `parties`-Liste
mit der Rolle `tenderer` identifizierbar. Bei Verfahren mit nur einem Bieter ist der
`tenderer` mit hoher Wahrscheinlichkeit der Zuschlagsempfänger. Nutze `has_awards=true`
für Releases, bei denen Lieferanten-Daten in `awards` vorhanden sind, oder alternativ
`tag="award"` allein, um alle Zuschlagsmeldungen zu finden (auch ohne Lieferanten).

**Beispiel: "Wer gewinnt die Ausschreibungen, die zu meinem Profil passen?"**
```
1. match_tenders(profile_id="...", k=30, tag="award", has_awards=true)
2. Für jeden Treffer: get_release(ocid="...")
3. → parties mit Rolle "tenderer" oder "supplier" = Gewinner/Mitbewerber
4. Häufigkeitsanalyse: Welche Firmen tauchen mehrfach auf?
```

---

## Tool-Referenz (10 Tools)

### Suche & Recherche
| Tool | Beschreibung |
|------|-------------|
| `search_text` | Semantische Textsuche: Anfrage wird lokal eingebettet und per KNN-Kosinus-Suche gegen Ausschreibungs-Chunks gematcht. Deutsche Anfragen liefern die besten Ergebnisse. |
| `list_releases` | Strukturierte Abfrage mit Filtern: Monat, CPV-Präfix, Kategorie, Vergabeverfahren, Status, Wertbereich, Auftraggeber, Fristbereich, Ergebniscode, NUTS-Code. Paginiert (limit/offset). Gibt Zusammenfassungen mit Gesamtzahl zurück. |
| `get_release` | Vollständiger OCDS-Release als JSON nach OCID. Enthält Tender, Auftraggeber, Parteien, Zuschläge, Lose, Positionen — alles. Jeder Release hat ein `url`-Feld mit einem Direktlink zur Ausschreibung auf oeffentlichevergabe.de. |

### Firmenprofile & Matching
| Tool | Beschreibung |
|------|-------------|
| `create_company_profile` | Firmenprofil speichern. Beschreibung wird automatisch eingebettet. Gibt UUID und Embedding-Status zurück. Deutsche Beschreibung empfohlen! |
| `update_company_profile` | Teilaktualisierung — nur übergebene Felder ändern sich. Bei Beschreibungsänderung wird neu eingebettet. |
| `get_company_profile` | Vollständiges Profil nach UUID. |
| `list_company_profiles` | Alle Profile, nach Erstellungsdatum sortiert. |
| `delete_company_profile` | Profil und Embedding löschen. |
| `match_tenders` | KNN-Matching: Profil-Embedding vs. Ausschreibungs-Chunks. Angereichert mit Release-Metadaten. Nachfilterbar nach CPV, Kategorie, Verfahren, Wert, Auftraggeber, Frist, Status, Tag, has_awards, eu_funded (EU-Förderung), location_nuts (Lieferort). Dedupliziert nach OCID. |

### Serverstatus
| Tool | Beschreibung |
|------|-------------|
| `get_index_info` | Kombinierte Statistiken: Ausschreibungs-/Embedding-Anzahl von der REST-API, lokale Profilanzahl, Embedder-Status, API-URL. Zuerst aufrufen! |

---

## Suchergebnisse verstehen

Suchergebnisse (`search_text`, `match_tenders`) liefern **Chunks**, keine
vollständigen Releases. Jede Ausschreibung wird für das Embedding in Chunks aufgeteilt:

| Chunk-Typ | Inhalt |
|-----------|--------|
| `summary` | Auftraggeber, Titel, Verfahren, Kategorie, Frist, NUTS-Code — kompakte Übersicht. |
| `tender` | Titel, vollständige Beschreibung, Verfahren, Kategorie, Wert, Frist, Auftraggeber, CPV-Bezeichnungen, Eignungskriterien, Zuschlagskriterien, Ergebniscode — der informationsreichste Chunk. |
| `lot:<id>` | Einzelnes Los: Titel, Beschreibung, Wert. Nur bei Ausschreibungen mit Losen. |

Jedes Ergebnis enthält: `ocid` (Verweis auf den vollständigen Release), `text`
(der Chunk-Text), `score` (Kosinus-Ähnlichkeit, 0–1), `chunk_type`, `cpv_codes`,
`url` (Direktlink zu oeffentlichevergabe.de) und ggf. `documents_url` (Link zu
den Vergabeunterlagen auf der eVergabe-Plattform).

Für den vollständigen Release hinter einem Suchergebnis: `get_release` mit der
`ocid` aufrufen.

---

## Datenquelle und Datenqualität

Die Daten stammen aus eForms-XML (UBL 2.3) von oeffentlichevergabe.de und werden
in das OCDS-Datenmodell überführt. Die eForms-Quelle liefert deutlich mehr Details
als die frühere OCDS-JSON-Variante.

### Fristen / Deadlines

Abgabefristen sind im Feld `submissionDeadline` verfügbar (ISO-8601 datetime).
Dieses Feld ist bei den meisten Ausschreibungen (ContractNotice) befüllt.
Nutze `deadline_before` / `deadline_after` in `list_releases` und `match_tenders`.

### Zuschlagsempfänger / Award Suppliers

Bei Zuschlagsinformationen (ContractAwardNotice) sind `awards[].suppliers` und
`awards[].value` häufiger befüllt als in der alten OCDS-JSON-Variante. Zusätzlich:

1. Prüfe `parties[]` — suche nach Einträgen mit der Rolle `tenderer` oder `supplier`.
2. Bei manchen Ausschreibungen sind Zuschlagswerte aus Vertraulichkeitsgründen
   nicht veröffentlicht (`FieldsPrivacy`).

### Eignungs- und Zuschlagskriterien

- `selectionCriteria[]` — Eignungskriterien mit Typ (`sui-act`, `tp-abil`, `ef-stand`)
  und Beschreibung. Zeigen, welche Qualifikationen ein Bieter nachweisen muss.
- `awardCriteria[]` — Zuschlagskriterien mit Name, Typ (`price`, `quality`, `cost`)
  und Gewichtung (%). Zeigen, wie Angebote bewertet werden.

### Ergebniscode / Result Code

Das Feld `resultCode` bei Zuschlagsentscheidungen zeigt:
- `selec-w` — Gewinner ausgewählt
- `clos-nw` — Abgeschlossen ohne Gewinner
- `open-nw` — Noch kein Gewinner

### NUTS-Codes

NUTS-Codes (z.B. `DE212` für München) identifizieren die geografische Region.
Filtere mit `nuts_code` Präfix, z.B. `DE2` für Bayern.

### Ausschreibungs-URLs

Jeder Release enthält ein `url`-Feld mit einem Direktlink zur Ausschreibung auf
oeffentlichevergabe.de. Diesen Link **immer dem Nutzer anzeigen** — er führt zur
vollständigen Ausschreibung mit allen Dokumenten, Fristen und Formularen.

Format: `https://oeffentlichevergabe.de/ui/de/search/details?noticeId={id}`

---

## Deutsches Vergaberecht — Kurzüberblick

Dieses Kapitel hilft, die Ausschreibungen im Kontext des deutschen Vergaberechts
einzuordnen und Nutzern bei der Bewerbungsvorbereitung zu helfen.

### Rechtsrahmen

| Ebene | Gesetz/Verordnung | Anwendung |
|-------|-------------------|-----------|
| EU-Schwellenwerte | GWB Teil 4 + VgV | Liefer-/Dienstleistungen ab 221.000 EUR, Bauleistungen ab 5.538.000 EUR |
| Unter Schwellenwert | UVgO (Liefer/DL), VOB/A (Bau) | Nationale Vergabe, weniger formale Anforderungen |

### Vergabeverfahren (procurementMethod)

| OCDS-Wert | Deutsches Verfahren | Bedeutung |
|-----------|-------------------|-----------|
| `open` | Offenes Verfahren | Jedes Unternehmen kann ein Angebot abgeben. Höchster Wettbewerb. |
| `selective` | Nichtoffenes Verfahren / Verhandlungsverfahren mit Teilnahmewettbewerb | Erst Teilnahmeantrag (Eignung), dann Angebotsaufforderung an qualifizierte Bieter. |
| `limited` | Verhandlungsverfahren ohne Teilnahmewettbewerb | Einladung ausgewählter Unternehmen. Nur in begründeten Ausnahmen. |
| `direct` | Direktvergabe | Einzelvergabe, kein Wettbewerb. Nur bei zwingenden Gründen (z.B. Herstellerexklusivität). |

### Typische Eignungsanforderungen

Wer sich auf eine öffentliche Ausschreibung bewerben will, muss typischerweise nachweisen:

**1. Eigenerklärungen (Pflicht)**
- Keine Ausschlussgründe nach §§ 123, 124 GWB (Straftaten, Insolvenz, Steuerschulden)
- Einhaltung Mindestlohn (MiLoG), Schwarzarbeitsbekämpfung (SchwarzArbG)
- Russland-Sanktionen (EU-Verordnung 833/2014, Art. 5k)

**2. Wirtschaftliche Leistungsfähigkeit (§ 45 VgV)**
- Jahresumsatz (Ø letzte 3 Jahre, max. das Zweifache des Auftragswerts)
- Betriebshaftpflichtversicherung
- Ggf. Jahresabschlüsse / Bankerklärungen

**3. Technische Leistungsfähigkeit (§ 46 VgV)**
- Referenzprojekte: Mind. 3 vergleichbare Aufträge der letzten 3 Jahre
- Qualifikationsprofile der eingesetzten Mitarbeiter (inkl. Zertifizierungen)
- Beschäftigtenzahlen
- Ggf. Qualitätsmanagement-Zertifikate (ISO 9001, ISO 27001)

**4. Technisches Konzept**
- Verständnis der Aufgabenstellung
- Lösungsansatz und Vorgehensmodell (agil / klassisch)
- Personalkonzept (Rollen, Qualifikation, Verfügbarkeit)
- Qualitätssicherungskonzept

### Formvorschriften

- **Elektronische Abgabe ist Pflicht** (über die jeweilige eVergabe-Plattform)
- **Textform** genügt in der Regel (keine qualifizierte elektronische Signatur nötig)
- Vorgegebene Formulare des Auftraggebers verwenden
- Preisblatt exakt in vorgegebener Struktur ausfüllen
- **Angebotsfrist ist absolut** — eine Sekunde zu spät = zwingender Ausschluss

### Häufige Fehler

- Fristversäumnis (zwingender Ausschluss, keine Nachfrist)
- Fehlende Preisangaben oder Änderung der Vergabeunterlagen
- Referenzen nicht vergleichbar (zu alt, falscher Bereich, zu klein)
- Mindestanforderungen nicht erfüllt (Mindestumsatz, Mindestkapazität)
- Eigene AGB beigefügt (= Änderung der Vergabeunterlagen = Ausschluss)

### Lose (lots)

Viele größere Ausschreibungen werden in Lose aufgeteilt. Jedes Los kann separat
angeboten werden. Lose haben eigene Werte, Beschreibungen und oft eigene
Mindestanforderungen (z.B. "mind. 440 Tagewerke/Jahr"). Prüfe `tender.lots[]`
in den Release-Daten.

### Präqualifikation (AVPQ)

Das Amtliche Verzeichnis Präqualifizierter Unternehmen (AVPQ) bei den IHKs
ermöglicht eine Vorab-Eignungsprüfung. Ein Eintrag gilt als vorläufiger
Eignungsnachweis und beschleunigt Bewerbungsprozesse.

---

## OCDS-Datenmodell

### Lebenszyklus eines Vergabeverfahrens

Ein Vergabeverfahren durchläuft mehrere Phasen, die als separate Bekanntmachungen
(Notices) veröffentlicht werden:

```
Vorinformation (PIN)  →  Ausschreibung (CN)  →  Zuschlag (CAN)
  tag: planning           tag: tender            tag: award
```

Alle Bekanntmachungen desselben Verfahrens teilen sich dieselbe `ocid`.
Im System werden sie als **Events** gespeichert und zu einem einzigen Release
zusammengeführt (Merge). Das bedeutet:

- **Ein Release pro OCID** — auch wenn mehrere Bekanntmachungen existieren.
- **`tag` ist eine Liste** — enthält alle Phasen, z.B. `["tender", "award"]`
  bedeutet: Ausschreibung UND Zuschlag liegen vor.
- **Felder werden überlagert** — spätere Bekanntmachungen aktualisieren vorhandene
  Felder (z.B. `status` wird von `active` auf `complete` gesetzt), aber leere
  Felder überschreiben keine vorhandenen Werte (z.B. `description` aus der
  Ausschreibung bleibt erhalten, auch wenn der Zuschlag keine Beschreibung enthält).
- **`awards`** stammen aus der Zuschlagsbekanntmachung (CAN).
- **`documentsUrl`** stammt typischerweise aus der Ausschreibung (CN) und bleibt
  auch nach dem Zuschlag erhalten.

**Praxisrelevanz:** Wenn `tag` sowohl `tender` als auch `award` enthält, ist das
Verfahren bereits abgeschlossen — eine Bewerbung ist nicht mehr möglich. Offene
Ausschreibungen erkennt man an `tag: ["tender"]` ohne `award` und
`status: "active"`.

### Entitätshierarchie

```
Release (zusammengeführte Projektion pro Vergabevorgang / OCID)
 ├── tag[]            Lebenszyklusphase(n): planning, tender, award
 ├── parties[]        Beteiligte Organisationen (Auftraggeber, Bieter, etc.)
 ├── buyer            Der Auftraggeber (Vergabestelle)
 ├── tender           Die Ausschreibung selbst
 │    ├── items[]     Was beschafft wird
 │    ├── lots[]      Lose (können separat angeboten werden)
 │    ├── value       Geschätzter Gesamtwert
 │    ├── tenderPeriod  Angebotsfrist (endDate = Abgabefrist)
 │    ├── classification  Primärer CPV-Code
 │    └── documentsUrl  Link zu den Vergabeunterlagen (wenn vorhanden)
 └── awards[]         Zuschlagsentscheidungen (aus CAN)
      ├── suppliers[] Zuschlagsempfänger (ACHTUNG: oft leer, siehe Datenqualität!)
      └── value       Zuschlagswert
```

### Release-Felder

| Feld | Beschreibung |
|------|-------------|
| `ocid` | Open Contracting ID — weltweit eindeutig. Format: `ocds-{prefix}-{id}`. **Primärschlüssel** über Releases hinweg. |
| `id` | Release-ID, eindeutig innerhalb des Vergabeverfahrens. |
| `date` | Veröffentlichungsdatum (ISO 8601). |
| `tag` | Lebenszyklusphase(n): `planning` (Planung), `tender` (Ausschreibung), `award` (Zuschlag), `contract` (Vertrag), `implementation` (Durchführung). |
| `language` | Normalerweise `DEU` (Deutsch). |
| `url` | Direktlink zur Ausschreibung auf oeffentlichevergabe.de. |

### Tender-Felder (Ausschreibung)

| Feld | Beschreibung |
|------|-------------|
| `title` | Kurztitel (Deutsch). |
| `description` | Detaillierte Leistungsbeschreibung (Deutsch). |
| `status` | `planning`, `active`, `cancelled`, `unsuccessful`, `complete`, `withdrawn`. |
| `procurementMethod` | `open` (offen), `selective` (nichtoffenes Verfahren), `limited` (Verhandlungsverfahren), `direct` (Direktvergabe). |
| `procurementMethodDetails` | Originaler eForms-Verfahrenscode, z.B. `de-open`, `neg-w-call`, `restricted`. Nützlich für die exakte Verfahrensbestimmung. |
| `mainProcurementCategory` | `goods` (Lieferungen), `works` (Bauleistungen) oder `services` (Dienstleistungen). |
| `value` | `{amount, currency}` — geschätzter Gesamtwert. Währung ist fast immer EUR. |
| `tenderPeriod` | `{startDate, endDate}` — endDate ist die Angebotsfrist. |
| `submissionDeadline` | Abgabefrist als ISO-8601 datetime (aus eForms). Bei den meisten Ausschreibungen vorhanden. |
| `classification` | Primärer CPV-Code `{scheme: "CPV", id, description}`. |
| `selectionCriteria` | Eignungskriterien: `{criterionType, description}`. Typen: `sui-act` (Befähigung), `tp-abil` (Leistungsfähigkeit), `ef-stand` (wirtschaftliche Leistungsfähigkeit). |
| `awardCriteria` | Zuschlagskriterien: `{name, criterionType, weight}`. Typen: `price`, `quality`, `cost`. Gewichtung in %. |
| `resultCode` | Ergebnis: `selec-w` (Gewinner), `clos-nw` (ohne Gewinner), `open-nw` (offen). |
| `location` | Lieferort: `{city, postal_code, nuts_code, country}`. NUTS-Code für regionale Filterung. |
| `duration` | Vertragslaufzeit: `{value, unit, start_date, end_date}`. Einheiten: `DAY`, `MONTH`, `YEAR`. |
| `euFunded` | `true` wenn EU-gefördert (z.B. Strukturfonds). Filterbar mit `eu_funded=true`. |
| `maxRenewals` | Maximale Anzahl Verlängerungen des Vertrags. |
| `maxLotsAwarded` | Maximale Anzahl Lose, die an einen Bieter vergeben werden. |
| `maxLotsSubmitted` | Maximale Anzahl Lose, auf die ein Bieter bieten darf. |
| `documentsUrl` | Link zu den Vergabeunterlagen auf der eVergabe-Plattform (z.B. Deutsche eVergabe, Vergabe.NRW). Nicht bei allen Ausschreibungen vorhanden (~20%). **Nicht identisch mit `url`** — `url` verweist auf oeffentlichevergabe.de (die Bekanntmachung), `documentsUrl` auf die Plattform mit den eigentlichen Unterlagen. |

### Lot-Felder (Lose)

| Feld | Beschreibung |
|------|-------------|
| `id` | Los-Kennung. |
| `title` / `description` | Los-Details (Deutsch). |
| `value` | Geschätzter Wert dieses Loses. |
| `location` | Lieferort des Loses: `{city, postal_code, nuts_code, country}`. |
| `duration` | Vertragslaufzeit des Loses. |
| `euFunded` | `true` wenn dieses Los EU-gefördert ist. |
| `maxRenewals` | Maximale Anzahl Verlängerungen für dieses Los. |

### Award-Felder (Zuschläge)

| Feld | Beschreibung |
|------|-------------|
| `status` | `pending`, `active` (Zuschlag erteilt), `unsuccessful` (kein Zuschlag), `cancelled` (aufgehoben). |
| `date` | Datum der Zuschlagsentscheidung. |
| `value` | Tatsächlicher Zuschlagswert (manchmal Platzhalterwert 1,00 EUR). |
| `suppliers` | Zuschlagsempfänger `{id, name}`. **ACHTUNG: Fast immer leer!** Stattdessen `parties` mit Rolle `tenderer` prüfen. |
| `relatedLots` | IDs der Lose, auf die sich der Zuschlag bezieht. |
| `subcontracting` | Unterauftrags-Code (z.B. `sub-val`, `sub-per`). |
| `submissionStatistics` | Eingegangene Angebote: `{code, count}`. Code z.B. `tenders` = Anzahl Angebote. |
| `fieldsPrivacy` | Vertraulichkeitsflags (z.B. `val-tot` = Gesamtwert vertraulich). |

### Parties-Felder (Beteiligte)

| Rolle | Bedeutung |
|-------|-----------|
| `procuringEntity` | Vergabestelle / Auftraggeber |
| `buyer` | Auftraggeber (oft identisch mit procuringEntity) |
| `supplier` | Zuschlagsempfänger / Lieferant |
| `tenderer` | Bieter |
| `reviewBody` | Nachprüfungsstelle (Vergabekammer) |
| `serv-prov` | Verfahrensberater / Dienstleister des Auftraggebers |
| `mediationBody` | Schlichtungsstelle |

Zusätzliche Felder aus eForms:
- `nutsCode` — NUTS-Regionscode (z.B. `DE212` für München)
- `companySize` — `sme` (KMU) oder `large` (Großunternehmen)
- `companyId` — Unternehmenskennung (z.B. USt-IdNr.)

### Item-Felder (Positionen)

| Feld | Beschreibung |
|------|-------------|
| `id` | Positionskennung. |
| `description` | Was beschafft wird (Deutsch). |
| `classification` | CPV-Code für diese Position. |
| `quantity` / `unit` | Menge und Einheit. |
| `relatedLot` | Zu welchem Los diese Position gehört. |

---

## CPV-Codes (Gemeinsames Vokabular für öffentliche Aufträge)

CPV-Codes klassifizieren den Beschaffungsgegenstand. Nutze CPV-Präfixe für
branchenweite Filterung.

```
XX______  Abteilung  (2 Stellen) — breiteste Kategorie
XXXX____  Gruppe     (4 Stellen)
XXXXX___  Klasse     (5 Stellen)
XXXXXXXX  Kategorie  (8 Stellen) — spezifischste Ebene
```

### Wichtige Abteilungen

| Code | Abteilung (DE) | Division (EN) |
|------|----------------|---------------|
| 03 | Land- und Forstwirtschaft, Fischerei | Agriculture, farming, fishing |
| 09 | Erdöl, Brennstoffe, Energie | Petroleum, fuel, energy |
| 14–15 | Bergbau, Lebensmittel | Mining, food products |
| 22 | Druckerzeugnisse | Printed matter |
| 30–32 | Büro-/IT-Ausstattung, Elektronik, Telekommunikation | Office/computer equipment, electronics, telecom |
| **33** | **Medizinische Geräte, Pharmazeutika** | **Medical equipment, pharmaceuticals** |
| 34 | Transportmittel | Transport equipment |
| 38 | Labor-, optische, Präzisionsinstrumente | Laboratory, optical, precision instruments |
| 39 | Möbel | Furniture |
| 42–44 | Industriemaschinen, Baumaterialien | Industrial machinery, construction materials |
| **45** | **Bauarbeiten** | **Construction work** |
| **48** | **Softwarepakete, IT-Systeme** | **Software packages, IT systems** |
| 50–51 | Reparatur/Wartung, Installation | Repair/maintenance, installation |
| 55 | Hotel, Gaststätten, Catering | Hotel, restaurant, catering |
| 60–63 | Transportdienste | Transport services |
| 64–67 | Telekommunikations-, Finanzdienstleistungen | Telecom, financial services |
| 70–71 | Immobilien, Architektur, Ingenieurwesen | Real estate, architecture, engineering |
| **72** | **IT-Dienstleistungen, Beratung, Softwareentwicklung** | **IT services, consulting, software development** |
| 73 | F&E-Dienstleistungen | R&D services |
| 75–77 | Öffentliche Verwaltung, Verteidigung, Bildung | Public admin, defence, education |
| **79** | **Unternehmensdienstleistungen (Recht, Buchhaltung, Beratung)** | **Business services (legal, accounting, consulting)** |
| 80 | Bildung, Ausbildung | Education, training |
| 85 | Gesundheit, Sozialwesen | Health, social work |
| **90** | **Abwasser, Abfall, Reinigung, Umwelt** | **Sewage, refuse, cleaning, environmental** |
| 92 | Freizeit, Kultur, Sport | Recreational, cultural, sporting |

**Tipp:** Mit `cpv_prefix: "45"` alle Bauausschreibungen filtern, mit
`cpv_prefix: "72"` alle IT-Dienstleistungen, mit `cpv_prefix: "33"` alle
Medizintechnik-Ausschreibungen.

---

## Praktische Tipps

1. **Immer auf Deutsch suchen** — Alle Titel und Beschreibungen sind auf Deutsch.
   Deutsche Suchbegriffe liefern deutlich bessere Ergebnisse als englische.
   Firmenbeschreibungen für Profile ebenfalls auf Deutsch verfassen.

2. **OCID ist der Primärschlüssel** — Dasselbe Vergabeverfahren kann in mehreren
   Releases erscheinen (Planung → Ausschreibung → Zuschlag). Mit der `ocid`
   lässt sich ein Verfahren über die Zeit verfolgen.

3. **Lebenszyklusphasen** über `tag`:
   - `planning` — Vorinformation, noch keine Angebote möglich.
   - `tender` — Ausschreibung läuft. `tenderPeriod.endDate` prüfen (falls vorhanden).
   - `award` — Zuschlag erteilt. `parties` nach Rolle `tenderer`/`supplier` prüfen.
   - `contract` / `implementation` — Vertrag geschlossen / in Ausführung.

4. **Wettbewerbsintensität** über `procurementMethod`:
   - `open` → höchster Wettbewerb, jedes Unternehmen kann bieten.
   - `selective` → Präqualifikation erforderlich.
   - `limited` → nur eingeladene Unternehmen.
   - `direct` → Einzelvergabe, kein Wettbewerb.

5. **Lose** — Viele Ausschreibungen sind in Lose aufgeteilt, die separat angeboten
   werden können. Jedes Los hat eigenen Wert und eigene Beschreibung. Such- und
   Match-Ergebnisse enthalten Los-Chunks (`lot:<id>`).

6. **Wertangaben** — `tender.value` ist der Schätzwert; `award.value` ist der
   tatsächliche Zuschlagswert (manchmal Platzhalterwert 1,00 EUR). Fast immer EUR.

7. **Fristen** — `submissionDeadline` enthält die Abgabefrist (ISO-8601 datetime)
   und ist bei den meisten Ausschreibungen vorhanden. Nutze `deadline_before` /
   `deadline_after` Filter. Für die vollständigen Vergabeunterlagen den Direktlink
   zu oeffentlichevergabe.de verwenden.

8. **Ausschreibungs-Links** — Jeder Release hat ein `url`-Feld mit Direktlink zu
   oeffentlichevergabe.de. Diesen Link **immer dem Nutzer anzeigen**, damit er
   die vollständigen Vergabeunterlagen, Fristen und Formulare abrufen kann.

9. **Zuschlagsempfänger finden** — `awards[].suppliers` enthält die Gewinner.
   Zusätzlich in `parties` nach Rollen `tenderer` oder `supplier` suchen.

10. **Eignungs- und Zuschlagskriterien** — `selectionCriteria` zeigt, welche
    Qualifikationen gefordert sind. `awardCriteria` zeigt, wie Angebote bewertet
    werden (inkl. Gewichtung). Diese Informationen helfen bei der Einschätzung,
    ob eine Bewerbung Aussicht auf Erfolg hat.

11. **EU-Förderung** — `eu_funded=true` filtert auf EU-geförderte Ausschreibungen
    (z.B. Strukturfonds). Nützlich für Unternehmen, die an EU-Projekten teilnehmen.

12. **Lieferort / Delivery Location** — `location_nuts` filtert nach NUTS-Code-Präfix
    des Lieferorts (z.B. `DE3` für Berlin, `DE21` für Oberbayern). Dies ist der
    Erfüllungsort, nicht der Sitz des Auftraggebers. Verfügbar in `tender.location`
    und `lots[].location`.

13. **Vergabeunterlagen** — Wenn vorhanden, enthält `documentsUrl` einen Direktlink
    zur eVergabe-Plattform mit den vollständigen Vergabeunterlagen (Leistungsbeschreibung,
    Preisblätter, Eignungsnachweise, Formulare). Diesen Link dem Nutzer anzeigen.
    **Achtung:** Die meisten Plattformen erfordern eine Registrierung/Anmeldung.

14. **Bewerbungsvorbereitung** — Dieser Server findet und bewertet Ausschreibungen.
    Die eigentliche Angebotserstellung (Arbeitsverzeichnis anlegen, Unterlagen
    zusammenstellen, Checklisten erstellen, Konzepte entwerfen) kann das LLM
    direkt auf Basis der Ausschreibungsdaten und Vergabeunterlagen unterstützen —
    dafür werden keine zusätzlichen MCP-Tools benötigt.

15. **REST-API-Abhängigkeit** — Dieser MCP-Server benötigt die laufende
    `Vergabe Dashboard API` für Suche, Release-Abruf und Listen. Nur die Firmenprofil-
    Verwaltung funktioniert ohne REST-API.
"#;
