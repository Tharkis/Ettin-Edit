# Sample data for tables

Made-up files for trying the [table tab](../../docs/manual/src/tables.md) and
for the tests. Every name, number, address and identifier in them is invented;
none of it describes a real person or organisation.

| File | What it shows |
| --- | --- |
| `patients.csv` | Dates written `DD/MM/YYYY`, codes with leading zeros, a value that isn't a number |
| `lab-results.hl7` | Three HL7 v2.5.1 ORU^R01 lab result messages: repeating identifiers, several OBX segments, a note with escape sequences |
| `orders.json` | Nested JSON: orders with a clinic object and a list of order lines |
| `appointments.xml` | XML with attributes, nested elements and a repeated element |
