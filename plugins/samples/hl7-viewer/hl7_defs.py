"""A data dictionary for HL7 version 2 messages, with v2.5.1 as the baseline.

Segments list their fields and data types list their components. The tables
below are written in a compact text form, one entry per line:

    fields:      number|name|type|optionality|repeat|max length|table|note
    components:  name|type|table

repeat is blank (does not repeat), Y (repeats without a limit) or a number
(repeats up to that many times). Blank table or note columns can be left
off. A note marks fields added after v2.5.1.

Look things up with segment(), field(), datatype(), component() or lookup();
they never fail: unknown segments, fields and types get a readable fallback.
"""

from collections import namedtuple

FieldDef = namedtuple("FieldDef", "segment number name type optionality repeat length table note known")
SegmentDef = namedtuple("SegmentDef", "name description fields known")
TypeDef = namedtuple("TypeDef", "code name components known")
ComponentDef = namedtuple("ComponentDef", "number name type table known")

VERSION = "2.5.1"

OPTIONALITY = {
    "R": "Required",
    "O": "Optional",
    "C": "Conditional",
    "B": "Backward compatible (kept for older versions)",
    "W": "Withdrawn",
    "X": "Not used",
}


def optionality_text(code):
    """"R" -> "Required (R)"; "" -> "Unknown"."""
    if not code:
        return "Unknown"
    name = OPTIONALITY.get(code)
    return "%s (%s)" % (name, code) if name else code


def repeat_text(repeat):
    """1 -> "No", 0 -> "Yes", 3 -> "Yes, up to 3 times", None -> "Unknown"."""
    if repeat is None:
        return "Unknown"
    if repeat == 1:
        return "No"
    if repeat == 0:
        return "Yes"
    return "Yes, up to %d times" % repeat


# ----------------------------------------------------------------------
# Data types

_TYPES = {}


def _type(code, name, spec=""):
    comps = []
    for i, line in enumerate(l for l in spec.strip().splitlines() if l.strip()):
        parts = [p.strip() for p in line.split("|")]
        parts += [""] * (3 - len(parts))
        comps.append(ComponentDef(i + 1, parts[0], parts[1], parts[2] or None, True))
    _TYPES[code] = TypeDef(code, name, comps, True)


# Primitive types: no components.
_type("ST", "String Data")
_type("TX", "Text Data")
_type("FT", "Formatted Text Data")
_type("NM", "Numeric")
_type("SI", "Sequence ID")
_type("ID", "Coded Value for HL7-Defined Tables")
_type("IS", "Coded Value for User-Defined Tables")
_type("DT", "Date")
_type("TM", "Time")
_type("DTM", "Date/Time")
_type("GTS", "General Timing Specification")
_type("varies", "Varies (the type is given elsewhere, such as in OBX-2)")

_type("TS", "Time Stamp", """
Time|DTM
Degree of Precision|ID|0529
""")
_type("CE", "Coded Element", """
Identifier|ST
Text|ST
Name of Coding System|ID|0396
Alternate Identifier|ST
Alternate Text|ST
Name of Alternate Coding System|ID|0396
""")
_CODED_EXTENDED = """
Identifier|ST
Text|ST
Name of Coding System|ID|0396
Alternate Identifier|ST
Alternate Text|ST
Name of Alternate Coding System|ID|0396
Coding System Version ID|ST
Alternate Coding System Version ID|ST
Original Text|ST
"""
_type("CWE", "Coded with Exceptions", _CODED_EXTENDED)
_type("CNE", "Coded with No Exceptions", _CODED_EXTENDED)
_type("CF", "Coded Element with Formatted Values", """
Identifier|ST
Formatted Text|FT
Name of Coding System|ID|0396
Alternate Identifier|ST
Alternate Formatted Text|FT
Name of Alternate Coding System|ID|0396
""")
_type("CX", "Extended Composite ID with Check Digit", """
ID Number|ST
Check Digit|ST
Check Digit Scheme|ID|0061
Assigning Authority|HD|0363
Identifier Type Code|ID|0203
Assigning Facility|HD
Effective Date|DT
Expiration Date|DT
Assigning Jurisdiction|CWE
Assigning Agency or Department|CWE
""")
_type("FN", "Family Name", """
Surname|ST
Own Surname Prefix|ST
Own Surname|ST
Surname Prefix from Partner/Spouse|ST
Surname from Partner/Spouse|ST
""")
_type("XPN", "Extended Person Name", """
Family Name|FN
Given Name|ST
Second and Further Given Names or Initials Thereof|ST
Suffix (e.g., JR or III)|ST
Prefix (e.g., DR)|ST
Degree (e.g., MD)|IS|0360
Name Type Code|ID|0200
Name Representation Code|ID|0465
Name Context|CE|0448
Name Validity Range|DR
Name Assembly Order|ID|0444
Effective Date|TS
Expiration Date|TS
Professional Suffix|ST
""")
_type("SAD", "Street Address", """
Street or Mailing Address|ST
Street Name|ST
Dwelling Number|ST
""")
_type("XAD", "Extended Address", """
Street Address|SAD
Other Designation|ST
City|ST
State or Province|ST
Zip or Postal Code|ST
Country|ID|0399
Address Type|ID|0190
Other Geographic Designation|ST
County/Parish Code|IS|0289
Census Tract|IS|0288
Address Representation Code|ID|0465
Address Validity Range|DR
Effective Date|TS
Expiration Date|TS
""")
_type("AD", "Address", """
Street Address|ST
Other Designation|ST
City|ST
State or Province|ST
Zip or Postal Code|ST
Country|ID|0399
Address Type|ID|0190
Other Geographic Designation|ST
""")
_type("XTN", "Extended Telecommunication Number", """
Telephone Number|ST
Telecommunication Use Code|ID|0201
Telecommunication Equipment Type|ID|0202
Email Address|ST
Country Code|NM
Area/City Code|NM
Local Number|NM
Extension|NM
Any Text|ST
Extension Prefix|ST
Speed Dial Code|ST
Unformatted Telephone Number|ST
""")
_type("XCN", "Extended Composite ID Number and Name for Persons", """
ID Number|ST
Family Name|FN
Given Name|ST
Second and Further Given Names or Initials Thereof|ST
Suffix (e.g., JR or III)|ST
Prefix (e.g., DR)|ST
Degree (e.g., MD)|IS|0360
Source Table|IS|0297
Assigning Authority|HD|0363
Name Type Code|ID|0200
Identifier Check Digit|ST
Check Digit Scheme|ID|0061
Identifier Type Code|ID|0203
Assigning Facility|HD
Name Representation Code|ID|0465
Name Context|CE|0448
Name Validity Range|DR
Name Assembly Order|ID|0444
Effective Date|TS
Expiration Date|TS
Professional Suffix|ST
Assigning Jurisdiction|CWE
Assigning Agency or Department|CWE
""")
_type("PPN", "Performing Person Time Stamp", """
ID Number|ST
Family Name|FN
Given Name|ST
Second and Further Given Names or Initials Thereof|ST
Suffix (e.g., JR or III)|ST
Prefix (e.g., DR)|ST
Degree (e.g., MD)|IS|0360
Source Table|IS|0297
Assigning Authority|HD|0363
Name Type Code|ID|0200
Identifier Check Digit|ST
Check Digit Scheme|ID|0061
Identifier Type Code|ID|0203
Assigning Facility|HD
Date/Time Action Performed|TS
Name Representation Code|ID|0465
Name Context|CE|0448
Name Validity Range|DR
Name Assembly Order|ID|0444
Effective Date|TS
Expiration Date|TS
Professional Suffix|ST
Assigning Jurisdiction|CWE
Assigning Agency or Department|CWE
""")
_type("CNN", "Composite ID Number and Name Simplified", """
ID Number|ST
Family Name|ST
Given Name|ST
Second and Further Given Names or Initials Thereof|ST
Suffix (e.g., JR or III)|ST
Prefix (e.g., DR)|ST
Degree (e.g., MD)|IS|0360
Source Table|IS|0297
Assigning Authority - Namespace ID|IS|0363
Assigning Authority - Universal ID|ST
Assigning Authority - Universal ID Type|ID|0301
""")
_type("XON", "Extended Composite Name and Identification Number for Organizations", """
Organization Name|ST
Organization Name Type Code|IS|0204
ID Number|NM
Check Digit|NM
Check Digit Scheme|ID|0061
Assigning Authority|HD|0363
Identifier Type Code|ID|0203
Assigning Facility|HD
Name Representation Code|ID|0465
Organization Identifier|ST
""")
_type("HD", "Hierarchic Designator", """
Namespace ID|IS|0300
Universal ID|ST
Universal ID Type|ID|0301
""")
_type("EI", "Entity Identifier", """
Entity Identifier|ST
Namespace ID|IS|0363
Universal ID|ST
Universal ID Type|ID|0301
""")
_type("EIP", "Entity Identifier Pair", """
Placer Assigned Identifier|EI
Filler Assigned Identifier|EI
""")
_type("PL", "Person Location", """
Point of Care|IS|0302
Room|IS|0303
Bed|IS|0304
Facility|HD
Location Status|IS|0306
Person Location Type|IS|0305
Building|IS|0307
Floor|IS|0308
Location Description|ST
Comprehensive Location Identifier|EI
Assigning Authority for Location|HD
""")
_type("MSG", "Message Type", """
Message Code|ID|0076
Trigger Event|ID|0003
Message Structure|ID|0354
""")
_type("PT", "Processing Type", """
Processing ID|ID|0103
Processing Mode|ID|0207
""")
_type("VID", "Version Identifier", """
Version ID|ID|0104
Internationalization Code|CE|0399
International Version ID|CE
""")
_type("CQ", "Composite Quantity with Units", """
Quantity|NM
Units|CE
""")
_type("MO", "Money", """
Quantity|NM
Denomination|ID|0913
""")
_type("CP", "Composite Price", """
Price|MO
Price Type|ID|0205
From Value|NM
To Value|NM
Range Units|CE
Range Type|ID|0298
""")
_type("MOC", "Money and Code", """
Monetary Amount|MO
Charge Code|CE
""")
_type("MOP", "Money or Percentage", """
Money or Percentage Indicator|ID|0148
Money or Percentage Quantity|NM
Currency Denomination|ID|0913
""")
_type("FC", "Financial Class", """
Financial Class Code|IS|0064
Effective Date|TS
""")
_type("DLN", "Driver's License Number", """
License Number|ST
Issuing State, Province, Country|IS|0333
Expiration Date|DT
""")
_type("JCC", "Job Code/Class", """
Job Code|IS|0327
Job Class|IS|0328
Job Description Text|TX
""")
_type("LA1", "Location with Address Variation 1", """
Point of Care|IS|0302
Room|IS|0303
Bed|IS|0304
Facility|HD
Location Status|IS|0306
Patient Location Type|IS|0305
Building|IS|0307
Floor|IS|0308
Address|AD
""")
_type("LA2", "Location with Address Variation 2", """
Point of Care|IS|0302
Room|IS|0303
Bed|IS|0304
Facility|HD
Location Status|IS|0306
Patient Location Type|IS|0305
Building|IS|0307
Floor|IS|0308
Street Address|ST
Other Designation|ST
City|ST
State or Province|ST
Zip or Postal Code|ST
Country|ID|0399
Address Type|ID|0190
Other Geographic Designation|ST
""")
_type("NDL", "Name with Date and Location", """
Name|CNN
Start Date/Time|TS
End Date/Time|TS
Point of Care|IS|0302
Room|IS|0303
Bed|IS|0304
Facility|HD
Location Status|IS|0306
Patient Location Type|IS|0305
Building|IS|0307
Floor|IS|0308
""")
_type("PRL", "Parent Result Link", """
Parent Observation Identifier|CE
Parent Observation Sub-identifier|ST
Parent Observation Value Descriptor|TX
""")
_type("SPS", "Specimen Source", """
Specimen Source Name or Code|CWE|0070
Additives|CWE|0371
Specimen Collection Method|TX
Body Site|CWE|0163
Site Modifier|CWE|0495
Collection Method Modifier Code|CWE
Specimen Role|CWE|0369
""")
_type("TQ", "Timing Quantity", """
Quantity|CQ
Interval|RI
Duration|ST
Start Date/Time|TS
End Date/Time|TS
Priority|ST
Condition|ST
Text|TX
Conjunction|ID|0472
Order Sequencing|OSD
Occurrence Duration|CE
Total Occurrences|NM
""")
_type("RI", "Repeat Interval", """
Repeat Pattern|IS|0335
Explicit Time Interval|ST
""")
_type("RPT", "Repeat Pattern", """
Repeat Pattern Code|CWE|0335
Calendar Alignment|ID|0527
Phase Range Begin Value|NM
Phase Range End Value|NM
Period Quantity|NM
Period Units|IS
Institution Specified Time|ID|0136
Event|ID|0528
Event Offset Quantity|NM
Event Offset Units|IS
General Timing Specification|GTS
""")
_type("OSD", "Order Sequence Definition", """
Sequence/Results Flag|ID|0524
Placer Order Number: Entity Identifier|ST
Placer Order Number: Namespace ID|IS
Filler Order Number: Entity Identifier|ST
Filler Order Number: Namespace ID|IS
Sequence Condition Value|ST
Maximum Number of Repeats|NM
Placer Order Number: Universal ID|ST
Placer Order Number: Universal ID Type|ID|0301
Filler Order Number: Universal ID|ST
Filler Order Number: Universal ID Type|ID|0301
""")
_type("ELD", "Error Location and Description", """
Segment ID|ST
Segment Sequence|NM
Field Position|NM
Code Identifying Error|CE|0357
""")
_type("ERL", "Error Location", """
Segment ID|ST
Segment Sequence|NM
Field Position|NM
Field Repetition|NM
Component Number|NM
Sub-Component Number|NM
""")
_type("CCD", "Charge Code and Date", """
Invocation Event|ID|0100
Date/Time|TS
""")
_type("DLD", "Discharge to Location and Date", """
Discharge to Location|IS|0113
Effective Date|TS
""")
_type("DR", "Date/Time Range", """
Range Start Date/Time|TS
Range End Date/Time|TS
""")
_type("SN", "Structured Numeric", """
Comparator|ST
Num1|NM
Separator/Suffix|ST
Num2|NM
""")
_type("NR", "Numeric Range", """
Low Value|NM
High Value|NM
""")
_type("ED", "Encapsulated Data", """
Source Application|HD
Type of Data|ID|0191
Data Subtype|ID|0291
Encoding|ID|0299
Data|ST
""")
_type("RP", "Reference Pointer", """
Pointer|ST
Application ID|HD
Type of Data|ID|0191
Subtype|ID|0291
""")
_type("PIP", "Practitioner Institutional Privileges", """
Privilege|CE|0525
Privilege Class|CE|0526
Expiration Date|DT
Activation Date|DT
Facility|EI
""")
_type("AUI", "Authorization Information", """
Authorization Number|ST
Date|DT
Source|ST
""")
_type("ICD", "Insurance Certification Definition", """
Certification Patient Type|IS|0150
Certification Required|ID|0136
Date/Time Certification Required|TS
""")
_type("VH", "Visiting Hours", """
Start Day Range|ID|0267
End Day Range|ID|0267
Start Hour Range|TM
End Hour Range|TM
""")
_type("CSU", "Channel Sensitivity and Units", """
Channel Sensitivity|NM
Unit of Measure Identifier|ST
Unit of Measure Description|ST
Unit of Measure Coding System|ID|0396
Alternate Unit of Measure Identifier|ST
Alternate Unit of Measure Description|ST
Alternate Unit of Measure Coding System|ID|0396
""")
_type("RMC", "Room Coverage", """
Room Type|IS|0145
Amount Type|IS|0146
Coverage Amount|NM
Money or Percentage|MOP
""")
_type("PTA", "Policy Type and Amount", """
Policy Type|IS|0147
Amount Class|IS|0193
Money or Percentage Quantity|NM
Money or Percentage|MOP
""")
_type("DDI", "Daily Deductible Information", """
Delay Days|NM
Monetary Amount|MO
Number of Days|NM
""")
_type("VR", "Value Range", """
First Data Code Value|ST
Last Data Code Value|ST
""")
_type("SRT", "Sort Order", """
Sort-by Field|ST
Sequencing|ID|0397
""")
_type("SCV", "Scheduling Class Value Pair", """
Parameter Class|CWE|0294
Parameter Value|ST
""")
_type("DIN", "Date and Institution Name", """
Date|TS
Institution Name|CE
""")
_type("SPD", "Specialty Description", """
Specialty Name|ST
Governing Board|ST
Eligible or Certified|ID|0337
Date of Certification|DT
""")
_type("TN", "Telephone Number")


# ----------------------------------------------------------------------
# Segments

_SEGMENTS = {}


def _repeat(text):
    if not text or text == "-":
        return 1
    if text.upper() == "Y":
        return 0
    return int(text)


def _segment(name, description, spec):
    fields = []
    for line in (l for l in spec.strip().splitlines() if l.strip()):
        parts = [p.strip() for p in line.split("|")]
        parts += [""] * (8 - len(parts))
        number = int(parts[0])
        if number != len(fields) + 1:
            raise ValueError("%s: field %d is out of order" % (name, number))
        fields.append(FieldDef(
            name, number, parts[1], parts[2], parts[3], _repeat(parts[4]),
            int(parts[5]) if parts[5] else None, parts[6] or None, parts[7] or None, True))
    _SEGMENTS[name] = SegmentDef(name, description, fields, True)


_segment("MSH", "Message Header", """
1|Field Separator|ST|R||1
2|Encoding Characters|ST|R||4
3|Sending Application|HD|O||227|0361
4|Sending Facility|HD|O||227|0362
5|Receiving Application|HD|O||227|0361
6|Receiving Facility|HD|O||227|0362
7|Date/Time of Message|TS|R||26
8|Security|ST|O||40
9|Message Type|MSG|R||15
10|Message Control ID|ST|R||20
11|Processing ID|PT|R||3
12|Version ID|VID|R||60
13|Sequence Number|NM|O||15
14|Continuation Pointer|ST|O||180
15|Accept Acknowledgment Type|ID|O||2|0155
16|Application Acknowledgment Type|ID|O||2|0155
17|Country Code|ID|O||3|0399
18|Character Set|ID|O|Y|16|0211
19|Principal Language of Message|CE|O||250
20|Alternate Character Set Handling Scheme|ID|O||20|0356
21|Message Profile Identifier|EI|O|Y|427
""")

_segment("SFT", "Software Segment", """
1|Software Vendor Organization|XON|R||567
2|Software Certified Version or Release Number|ST|R||15
3|Software Product Name|ST|R||20
4|Software Binary ID|ST|R||20
5|Software Product Information|TX|O||1024
6|Software Install Date|TS|O||26
""")

_segment("EVN", "Event Type", """
1|Event Type Code|ID|B||3|0003
2|Recorded Date/Time|TS|R||26
3|Date/Time Planned Event|TS|O||26
4|Event Reason Code|IS|O||3|0062
5|Operator ID|XCN|O|Y|250|0188
6|Event Occurred|TS|O||26
7|Event Facility|HD|O||241
""")

_segment("PID", "Patient Identification", """
1|Set ID - PID|SI|O||4
2|Patient ID|CX|B||20
3|Patient Identifier List|CX|R|Y|250
4|Alternate Patient ID - PID|CX|B|Y|20
5|Patient Name|XPN|R|Y|250
6|Mother's Maiden Name|XPN|O|Y|250
7|Date/Time of Birth|TS|O||26
8|Administrative Sex|IS|O||1|0001
9|Patient Alias|XPN|B|Y|250
10|Race|CE|O|Y|250|0005
11|Patient Address|XAD|O|Y|250
12|County Code|IS|B||4|0289
13|Phone Number - Home|XTN|O|Y|250
14|Phone Number - Business|XTN|O|Y|250
15|Primary Language|CE|O||250|0296
16|Marital Status|CE|O||250|0002
17|Religion|CE|O||250|0006
18|Patient Account Number|CX|O||250
19|SSN Number - Patient|ST|B||16
20|Driver's License Number - Patient|DLN|B||25
21|Mother's Identifier|CX|O|Y|250
22|Ethnic Group|CE|O|Y|250|0189
23|Birth Place|ST|O||250
24|Multiple Birth Indicator|ID|O||1|0136
25|Birth Order|NM|O||2
26|Citizenship|CE|O|Y|250|0171
27|Veterans Military Status|CE|O||250|0172
28|Nationality|CE|B||250|0212
29|Patient Death Date and Time|TS|O||26
30|Patient Death Indicator|ID|O||1|0136
31|Identity Unknown Indicator|ID|O||1|0136
32|Identity Reliability Code|IS|O|Y|20|0445
33|Last Update Date/Time|TS|O||26
34|Last Update Facility|HD|O||241
35|Species Code|CE|C||250|0446
36|Breed Code|CE|C||250|0447
37|Strain|ST|O||80
38|Production Class Code|CE|O|2|250|0429
39|Tribal Citizenship|CWE|O|Y|250|0171
""")

_segment("PD1", "Patient Additional Demographic", """
1|Living Dependency|IS|O|Y|2|0223
2|Living Arrangement|IS|O||2|0220
3|Patient Primary Facility|XON|O|Y|250
4|Patient Primary Care Provider Name & ID No.|XCN|B|Y|250
5|Student Indicator|IS|O||2|0231
6|Handicap|IS|O||2|0295
7|Living Will Code|IS|O||2|0315
8|Organ Donor Code|IS|O||2|0316
9|Separate Bill|ID|O||1|0136
10|Duplicate Patient|CX|O|Y|250
11|Publicity Code|CE|O||250|0215
12|Protection Indicator|ID|O||1|0136
13|Protection Indicator Effective Date|DT|O||8
14|Place of Worship|XON|O|Y|250
15|Advance Directive Code|CE|O|Y|250|0435
16|Immunization Registry Status|IS|O||1|0441
17|Immunization Registry Status Effective Date|DT|O||8
18|Publicity Code Effective Date|DT|O||8
19|Military Branch|IS|O||5|0140
20|Military Rank/Grade|IS|O||2|0141
21|Military Status|IS|O||3|0142
""")

_segment("NK1", "Next of Kin / Associated Parties", """
1|Set ID - NK1|SI|R||4
2|Name|XPN|O|Y|250
3|Relationship|CE|O||250|0063
4|Address|XAD|O|Y|250
5|Phone Number|XTN|O|Y|250
6|Business Phone Number|XTN|O|Y|250
7|Contact Role|CE|O||250|0131
8|Start Date|DT|O||8
9|End Date|DT|O||8
10|Next of Kin / Associated Parties Job Title|ST|O||60
11|Next of Kin / Associated Parties Job Code/Class|JCC|O||20
12|Next of Kin / Associated Parties Employee Number|CX|O||250
13|Organization Name - NK1|XON|O|Y|250
14|Marital Status|CE|O||250|0002
15|Administrative Sex|IS|O||1|0001
16|Date/Time of Birth|TS|O||26
17|Living Dependency|IS|O|Y|2|0223
18|Ambulatory Status|IS|O|Y|2|0009
19|Citizenship|CE|O|Y|250|0171
20|Primary Language|CE|O||250|0296
21|Living Arrangement|IS|O||2|0220
22|Publicity Code|CE|O||250|0215
23|Protection Indicator|ID|O||1|0136
24|Student Indicator|IS|O||2|0231
25|Religion|CE|O||250|0006
26|Mother's Maiden Name|XPN|O|Y|250
27|Nationality|CE|O||250|0212
28|Ethnic Group|CE|O|Y|250|0189
29|Contact Reason|CE|O|Y|250|0222
30|Contact Person's Name|XPN|O|Y|250
31|Contact Person's Telephone Number|XTN|O|Y|250
32|Contact Person's Address|XAD|O|Y|250
33|Next of Kin/Associated Party's Identifiers|CX|O|Y|250
34|Job Status|IS|O||2|0311
35|Race|CE|O|Y|250|0005
36|Handicap|IS|O||2|0295
37|Contact Person Social Security Number|ST|O||16
38|Next of Kin Birth Place|ST|O||250
39|VIP Indicator|IS|O||2|0099
""")

_segment("PV1", "Patient Visit", """
1|Set ID - PV1|SI|O||4
2|Patient Class|IS|R||1|0004
3|Assigned Patient Location|PL|O||80
4|Admission Type|IS|O||2|0007
5|Preadmit Number|CX|O||250
6|Prior Patient Location|PL|O||80
7|Attending Doctor|XCN|O|Y|250|0010
8|Referring Doctor|XCN|O|Y|250|0010
9|Consulting Doctor|XCN|B|Y|250|0010
10|Hospital Service|IS|O||3|0069
11|Temporary Location|PL|O||80
12|Preadmit Test Indicator|IS|O||2|0087
13|Re-admission Indicator|IS|O||2|0092
14|Admit Source|IS|O||6|0023
15|Ambulatory Status|IS|O|Y|2|0009
16|VIP Indicator|IS|O||2|0099
17|Admitting Doctor|XCN|O|Y|250|0010
18|Patient Type|IS|O||2|0018
19|Visit Number|CX|O||250
20|Financial Class|FC|O|Y|50|0064
21|Charge Price Indicator|IS|O||2|0032
22|Courtesy Code|IS|O||2|0045
23|Credit Rating|IS|O||2|0046
24|Contract Code|IS|O|Y|2|0044
25|Contract Effective Date|DT|O|Y|8
26|Contract Amount|NM|O|Y|12
27|Contract Period|NM|O|Y|3
28|Interest Code|IS|O||2|0073
29|Transfer to Bad Debt Code|IS|O||4|0110
30|Transfer to Bad Debt Date|DT|O||8
31|Bad Debt Agency Code|IS|O||10|0021
32|Bad Debt Transfer Amount|NM|O||12
33|Bad Debt Recovery Amount|NM|O||12
34|Delete Account Indicator|IS|O||1|0111
35|Delete Account Date|DT|O||8
36|Discharge Disposition|IS|O||3|0112
37|Discharged to Location|DLD|O||47|0113
38|Diet Type|CE|O||250|0114
39|Servicing Facility|IS|O||2|0115
40|Bed Status|IS|B||1|0116
41|Account Status|IS|O||2|0117
42|Pending Location|PL|O||80
43|Prior Temporary Location|PL|O||80
44|Admit Date/Time|TS|O||26
45|Discharge Date/Time|TS|O||26
46|Current Patient Balance|NM|O||12
47|Total Charges|NM|O||12
48|Total Adjustments|NM|O||12
49|Total Payments|NM|O||12
50|Alternate Visit ID|CX|O||250|0203
51|Visit Indicator|IS|O||1|0326
52|Other Healthcare Provider|XCN|B|Y|250|0010
""")

_segment("PV2", "Patient Visit - Additional Information", """
1|Prior Pending Location|PL|C||80
2|Accommodation Code|CE|O||250|0129
3|Admit Reason|CE|O||250
4|Transfer Reason|CE|O||250
5|Patient Valuables|ST|O|Y|25
6|Patient Valuables Location|ST|O||25
7|Visit User Code|IS|O|Y|2|0130
8|Expected Admit Date/Time|TS|O||26
9|Expected Discharge Date/Time|TS|O||26
10|Estimated Length of Inpatient Stay|NM|O||3
11|Actual Length of Inpatient Stay|NM|O||3
12|Visit Description|ST|O||50
13|Referral Source Code|XCN|O|Y|250
14|Previous Service Date|DT|O||8
15|Employment Illness Related Indicator|ID|O||1|0136
16|Purge Status Code|IS|O||1|0213
17|Purge Status Date|DT|O||8
18|Special Program Code|IS|O||2|0214
19|Retention Indicator|ID|O||1|0136
20|Expected Number of Insurance Plans|NM|O||1
21|Visit Publicity Code|IS|O||1|0215
22|Visit Protection Indicator|ID|O||1|0136
23|Clinic Organization Name|XON|O|Y|250
24|Patient Status Code|IS|O||2|0216
25|Visit Priority Code|IS|O||1|0217
26|Previous Treatment Date|DT|O||8
27|Expected Discharge Disposition|IS|O||2|0112
28|Signature on File Date|DT|O||8
29|First Similar Illness Date|DT|O||8
30|Patient Charge Adjustment Code|CE|O||250|0218
31|Recurring Service Code|IS|O||2|0219
32|Billing Media Code|ID|O||1|0136
33|Expected Surgery Date and Time|TS|O||26
34|Military Partnership Code|ID|O||1|0136
35|Military Non-Availability Code|ID|O||1|0136
36|Newborn Baby Indicator|ID|O||1|0136
37|Baby Detained Indicator|ID|O||1|0136
38|Mode of Arrival Code|CE|O||250|0430
39|Recreational Drug Use Code|CE|O|Y|250|0431
40|Admission Level of Care Code|CE|O||250|0432
41|Precaution Code|CE|O|Y|250|0433
42|Patient Condition Code|CE|O||250|0434
43|Living Will Code|IS|O||2|0315
44|Organ Donor Code|IS|O||2|0316
45|Advance Directive Code|CE|O|Y|250|0435
46|Patient Status Effective Date|DT|O||8
47|Expected LOA Return Date/Time|TS|C||26
48|Expected Pre-admission Testing Date/Time|TS|O||26
49|Notify Clergy Code|IS|O|Y|20|0534
""")

_segment("MRG", "Merge Patient Information", """
1|Prior Patient Identifier List|CX|R|Y|250
2|Prior Alternate Patient ID|CX|B|Y|250
3|Prior Patient Account Number|CX|O||250
4|Prior Patient ID|CX|B||250
5|Prior Visit Number|CX|O||250
6|Prior Alternate Visit ID|CX|O||250
7|Prior Patient Name|XPN|O|Y|250
""")

_segment("AL1", "Patient Allergy Information", """
1|Set ID - AL1|SI|R||4
2|Allergen Type Code|CE|O||250|0127
3|Allergen Code/Mnemonic/Description|CE|R||250
4|Allergy Severity Code|CE|O||250|0128
5|Allergy Reaction Code|ST|O|Y|15
6|Identification Date|DT|B||8
""")

_segment("IAM", "Patient Adverse Reaction Information", """
1|Set ID - IAM|SI|R||4
2|Allergen Type Code|CE|O||250|0127
3|Allergen Code/Mnemonic/Description|CE|R||250
4|Allergy Severity Code|CE|O||250|0128
5|Allergy Reaction Code|ST|O|Y|15
6|Allergy Action Code|CNE|R||250|0323
7|Allergy Unique Identifier|EI|C||427
8|Action Reason|ST|O||60
9|Sensitivity to Causative Agent Code|CE|O||250|0436
10|Allergen Group Code/Mnemonic/Description|CE|O||250
11|Onset Date|DT|O||8
12|Onset Date Text|ST|O||60
13|Reported Date/Time|TS|O||8
14|Reported By|XPN|O||250
15|Relationship to Patient Code|CE|O||250|0063
16|Alert Device Code|CE|O||250|0437
17|Allergy Clinical Status Code|CE|O||250|0438
18|Statused by Person|XCN|O||250
19|Statused by Organization|XON|O||250
20|Statused at Date/Time|TS|O||8
""")

_segment("DG1", "Diagnosis", """
1|Set ID - DG1|SI|R||4
2|Diagnosis Coding Method|ID|B||2|0053
3|Diagnosis Code - DG1|CE|O||250|0051
4|Diagnosis Description|ST|B||40
5|Diagnosis Date/Time|TS|O||26
6|Diagnosis Type|IS|R||2|0052
7|Major Diagnostic Category|CE|B||250|0118
8|Diagnostic Related Group|CE|B||250|0055
9|DRG Approval Indicator|ID|B||1|0136
10|DRG Grouper Review Code|IS|B||2|0056
11|Outlier Type|CE|B||250|0083
12|Outlier Days|NM|B||3
13|Outlier Cost|CP|B||12
14|Grouper Version and Type|ST|B||4
15|Diagnosis Priority|ID|O||2|0359
16|Diagnosing Clinician|XCN|O|Y|250
17|Diagnosis Classification|IS|O||3|0228
18|Confidential Indicator|ID|O||1|0136
19|Attestation Date/Time|TS|O||26
20|Diagnosis Identifier|EI|C||427
21|Diagnosis Action Code|ID|C||1|0206
""")

_segment("PR1", "Procedures", """
1|Set ID - PR1|SI|R||4
2|Procedure Coding Method|IS|B||3|0089
3|Procedure Code|CE|R||250|0088
4|Procedure Description|ST|B||40
5|Procedure Date/Time|TS|R||26
6|Procedure Functional Type|IS|O||2|0230
7|Procedure Minutes|NM|O||4
8|Anesthesiologist|XCN|B|Y|250|0010
9|Anesthesia Code|IS|O||2|0019
10|Anesthesia Minutes|NM|O||4
11|Surgeon|XCN|B|Y|250|0010
12|Procedure Practitioner|XCN|B|Y|250|0010
13|Consent Code|CE|O||250|0059
14|Procedure Priority|ID|O||2|0418
15|Associated Diagnosis Code|CE|O||250|0051
16|Procedure Code Modifier|CE|O|Y|250|0340
17|Procedure DRG Type|IS|O||20|0416
18|Tissue Type Code|CE|O|Y|250|0417
19|Procedure Identifier|EI|C||427
20|Procedure Action Code|ID|C||1|0206
""")

_segment("GT1", "Guarantor", """
1|Set ID - GT1|SI|R||4
2|Guarantor Number|CX|O|Y|250
3|Guarantor Name|XPN|R|Y|250
4|Guarantor Spouse Name|XPN|O|Y|250
5|Guarantor Address|XAD|O|Y|250
6|Guarantor Ph Num - Home|XTN|O|Y|250
7|Guarantor Ph Num - Business|XTN|O|Y|250
8|Guarantor Date/Time of Birth|TS|O||26
9|Guarantor Administrative Sex|IS|O||1|0001
10|Guarantor Type|IS|O||2|0068
11|Guarantor Relationship|CE|O||250|0063
12|Guarantor SSN|ST|O||11
13|Guarantor Date - Begin|DT|O||8
14|Guarantor Date - End|DT|O||8
15|Guarantor Priority|NM|O||2
16|Guarantor Employer Name|XPN|O|Y|250
17|Guarantor Employer Address|XAD|O|Y|250
18|Guarantor Employer Phone Number|XTN|O|Y|250
19|Guarantor Employee ID Number|CX|O|Y|250
20|Guarantor Employment Status|IS|O||2|0066
21|Guarantor Organization Name|XON|O|Y|250
22|Guarantor Billing Hold Flag|ID|O||1|0136
23|Guarantor Credit Rating Code|CE|O||250|0341
24|Guarantor Death Date and Time|TS|O||26
25|Guarantor Death Flag|ID|O||1|0136
26|Guarantor Charge Adjustment Code|CE|O||250|0218
27|Guarantor Household Annual Income|CP|O||10
28|Guarantor Household Size|NM|O||3
29|Guarantor Employer ID Number|CX|O|Y|250
30|Guarantor Marital Status Code|CE|O||250|0002
31|Guarantor Hire Effective Date|DT|O||8
32|Employment Stop Date|DT|O||8
33|Living Dependency|IS|O||2|0223
34|Ambulatory Status|IS|O|Y|2|0009
35|Citizenship|CE|O|Y|250|0171
36|Primary Language|CE|O||250|0296
37|Living Arrangement|IS|O||2|0220
38|Publicity Code|CE|O||250|0215
39|Protection Indicator|ID|O||1|0136
40|Student Indicator|IS|O||2|0231
41|Religion|CE|O||250|0006
42|Mother's Maiden Name|XPN|O|Y|250
43|Nationality|CE|O||250|0212
44|Ethnic Group|CE|O|Y|250|0189
45|Contact Person's Name|XPN|O|Y|250
46|Contact Person's Telephone Number|XTN|O|Y|250
47|Contact Reason|CE|O||250|0222
48|Contact Relationship|IS|O||3|0063
49|Job Title|ST|O||20
50|Job Code/Class|JCC|O||20
51|Guarantor Employer's Organization Name|XON|O|Y|250
52|Handicap|IS|O||2|0295
53|Job Status|IS|O||2|0311
54|Guarantor Financial Class|FC|O||50|0064
55|Guarantor Race|CE|O|Y|250|0005
56|Guarantor Birth Place|ST|O||250
57|VIP Indicator|IS|O||2|0099
""")

_segment("IN1", "Insurance", """
1|Set ID - IN1|SI|R||4
2|Insurance Plan ID|CE|R||250|0072
3|Insurance Company ID|CX|R|Y|250
4|Insurance Company Name|XON|O|Y|250
5|Insurance Company Address|XAD|O|Y|250
6|Insurance Co Contact Person|XPN|O|Y|250
7|Insurance Co Phone Number|XTN|O|Y|250
8|Group Number|ST|O||12
9|Group Name|XON|O|Y|250
10|Insured's Group Emp ID|CX|O|Y|250
11|Insured's Group Emp Name|XON|O|Y|250
12|Plan Effective Date|DT|O||8
13|Plan Expiration Date|DT|O||8
14|Authorization Information|AUI|O||239
15|Plan Type|IS|O||3|0086
16|Name of Insured|XPN|O|Y|250
17|Insured's Relationship to Patient|CE|O||250|0063
18|Insured's Date of Birth|TS|O||26
19|Insured's Address|XAD|O|Y|250
20|Assignment of Benefits|IS|O||2|0135
21|Coordination of Benefits|IS|O||2|0173
22|Coord of Ben. Priority|ST|O||2
23|Notice of Admission Flag|ID|O||1|0136
24|Notice of Admission Date|DT|O||8
25|Report of Eligibility Flag|ID|O||1|0136
26|Report of Eligibility Date|DT|O||8
27|Release Information Code|IS|O||2|0093
28|Pre-Admit Cert (PAC)|ST|O||15
29|Verification Date/Time|TS|O||26
30|Verification By|XCN|O|Y|250
31|Type of Agreement Code|IS|O||2|0098
32|Billing Status|IS|O||2|0022
33|Lifetime Reserve Days|NM|O||4
34|Delay Before L.R. Day|NM|O||4
35|Company Plan Code|IS|O||8|0042
36|Policy Number|ST|O||15
37|Policy Deductible|CP|O||12
38|Policy Limit - Amount|CP|B||12
39|Policy Limit - Days|NM|O||4
40|Room Rate - Semi-Private|CP|B||12
41|Room Rate - Private|CP|B||12
42|Insured's Employment Status|CE|O||250|0066
43|Insured's Administrative Sex|IS|O||1|0001
44|Insured's Employer's Address|XAD|O|Y|250
45|Verification Status|ST|O||2
46|Prior Insurance Plan ID|IS|O||8|0072
47|Coverage Type|IS|O||3|0309
48|Handicap|IS|O||2|0295
49|Insured's ID Number|CX|O|Y|250
50|Signature Code|IS|O||1|0535
51|Signature Code Date|DT|O||8
52|Insured's Birth Place|ST|O||250
53|VIP Indicator|IS|O||2|0099
""")

_segment("IN2", "Insurance Additional Information", """
1|Insured's Employee ID|CX|O|Y|250
2|Insured's Social Security Number|ST|O||11
3|Insured's Employer's Name and ID|XCN|O|Y|250
4|Employer Information Data|IS|O||1|0139
5|Mail Claim Party|IS|O|Y|1|0137
6|Medicare Health Ins Card Number|ST|O||15
7|Medicaid Case Name|XPN|O|Y|250
8|Medicaid Case Number|ST|O||15
9|Military Sponsor Name|XPN|O|Y|250
10|Military ID Number|ST|O||20
11|Dependent of Military Recipient|CE|O||250|0342
12|Military Organization|ST|O||25
13|Military Station|ST|O||25
14|Military Service|IS|O||14|0140
15|Military Rank/Grade|IS|O||2|0141
16|Military Status|IS|O||3|0142
17|Military Retire Date|DT|O||8
18|Military Non-Avail Cert on File|ID|O||1|0136
19|Baby Coverage|ID|O||1|0136
20|Combine Baby Bill|ID|O||1|0136
21|Blood Deductible|ST|O||1
22|Special Coverage Approval Name|XPN|O|Y|250
23|Special Coverage Approval Title|ST|O||30
24|Non-Covered Insurance Code|IS|O|Y|8|0143
25|Payor ID|CX|O|Y|250
26|Payor Subscriber ID|CX|O|Y|250
27|Eligibility Source|IS|O||1|0144
28|Room Coverage Type/Amount|RMC|O|Y|82
29|Policy Type/Amount|PTA|O|Y|56
30|Daily Deductible|DDI|O||25
31|Living Dependency|IS|O||2|0223
32|Ambulatory Status|IS|O|Y|2|0009
33|Citizenship|CE|O|Y|250|0171
34|Primary Language|CE|O||250|0296
35|Living Arrangement|IS|O||2|0220
36|Publicity Code|CE|O||250|0215
37|Protection Indicator|ID|O||1|0136
38|Student Indicator|IS|O||2|0231
39|Religion|CE|O||250|0006
40|Mother's Maiden Name|XPN|O|Y|250
41|Nationality|CE|O||250|0212
42|Ethnic Group|CE|O|Y|250|0189
43|Marital Status|CE|O|Y|250|0002
44|Insured's Employment Start Date|DT|O||8
45|Employment Stop Date|DT|O||8
46|Job Title|ST|O||20
47|Job Code/Class|JCC|O||20
48|Job Status|IS|O||2|0311
49|Employer Contact Person Name|XPN|O|Y|250
50|Employer Contact Person Phone Number|XTN|O|Y|250
51|Employer Contact Reason|IS|O||2|0222
52|Insured's Contact Person's Name|XPN|O|Y|250
53|Insured's Contact Person Phone Number|XTN|O|Y|250
54|Insured's Contact Person Reason|IS|O|Y|2|0222
55|Relationship to the Patient Start Date|DT|O||8
56|Relationship to the Patient Stop Date|DT|O|Y|8
57|Insurance Co. Contact Reason|IS|O||2|0232
58|Insurance Co Contact Phone Number|XTN|O||250
59|Policy Scope|IS|O||2|0312
60|Policy Source|IS|O||2|0313
61|Patient Member Number|CX|O||250
62|Guarantor's Relationship to Insured|CE|O||250|0063
63|Insured's Phone Number - Home|XTN|O|Y|250
64|Insured's Employer Phone Number|XTN|O|Y|250
65|Military Handicapped Program|CE|O||250|0343
66|Suspend Flag|ID|O||1|0136
67|Copay Limit Flag|ID|O||1|0136
68|Stoploss Limit Flag|ID|O||1|0136
69|Insured Organization Name and ID|XON|O|Y|250
70|Insured Employer Organization Name and ID|XON|O|Y|250
71|Race|CE|O|Y|250|0005
72|CMS Patient's Relationship to Insured|CE|O||250|0344
""")

_segment("ROL", "Role", """
1|Role Instance ID|EI|C||60
2|Action Code|ID|R||2|0287
3|Role-ROL|CE|R||250|0443
4|Role Person|XCN|R|Y|250
5|Role Begin Date/Time|TS|O||26
6|Role End Date/Time|TS|O||26
7|Role Duration|CE|O||250
8|Role Action Reason|CE|O||250
9|Provider Type|CE|O|Y|250
10|Organization Unit Type|CE|O||250|0406
11|Office/Home Address/Birthplace|XAD|O|Y|250
12|Phone|XTN|O|Y|250
""")

_segment("ORC", "Common Order", """
1|Order Control|ID|R||2|0119
2|Placer Order Number|EI|C||22
3|Filler Order Number|EI|C||22
4|Placer Group Number|EI|O||22
5|Order Status|ID|O||2|0038
6|Response Flag|ID|O||1|0121
7|Quantity/Timing|TQ|B|Y|200
8|Parent|EIP|O||200
9|Date/Time of Transaction|TS|O||26
10|Entered By|XCN|O|Y|250
11|Verified By|XCN|O|Y|250
12|Ordering Provider|XCN|O|Y|250
13|Enterer's Location|PL|O||80
14|Call Back Phone Number|XTN|O|2|250
15|Order Effective Date/Time|TS|O||26
16|Order Control Code Reason|CE|O||250
17|Entering Organization|CE|O||250
18|Entering Device|CE|O||250
19|Action By|XCN|O|Y|250
20|Advanced Beneficiary Notice Code|CE|O||250|0339
21|Ordering Facility Name|XON|O|Y|250
22|Ordering Facility Address|XAD|O|Y|250
23|Ordering Facility Phone Number|XTN|O|Y|250
24|Ordering Provider Address|XAD|O|Y|250
25|Order Status Modifier|CWE|O||250
26|Advanced Beneficiary Notice Override Reason|CWE|C||60|0552
27|Filler's Expected Availability Date/Time|TS|O||26
28|Confidentiality Code|CWE|O||250|0177
29|Order Type|CWE|O||250|0482
30|Enterer Authorization Mode|CNE|O||250|0483
31|Parent Universal Service Identifier|CWE|O||250
""")

_segment("OBR", "Observation Request", """
1|Set ID - OBR|SI|O||4
2|Placer Order Number|EI|C||22
3|Filler Order Number|EI|C||22
4|Universal Service Identifier|CE|R||250
5|Priority - OBR|ID|B||2
6|Requested Date/Time|TS|B||26
7|Observation Date/Time|TS|C||26
8|Observation End Date/Time|TS|O||26
9|Collection Volume|CQ|O||20
10|Collector Identifier|XCN|O|Y|250
11|Specimen Action Code|ID|O||1|0065
12|Danger Code|CE|O||250
13|Relevant Clinical Information|ST|O||300
14|Specimen Received Date/Time|TS|B||26
15|Specimen Source|SPS|B||300
16|Ordering Provider|XCN|O|Y|250
17|Order Callback Phone Number|XTN|O|2|250
18|Placer Field 1|ST|O||60
19|Placer Field 2|ST|O||60
20|Filler Field 1|ST|O||60
21|Filler Field 2|ST|O||60
22|Results Rpt/Status Chng - Date/Time|TS|C||26
23|Charge to Practice|MOC|O||40
24|Diagnostic Serv Sect ID|ID|O||10|0074
25|Result Status|ID|C||1|0123
26|Parent Result|PRL|O||400
27|Quantity/Timing|TQ|B|Y|200
28|Result Copies To|XCN|O|Y|250
29|Parent|EIP|O||200
30|Transportation Mode|ID|O||20|0124
31|Reason for Study|CE|O|Y|250
32|Principal Result Interpreter|NDL|O||200
33|Assistant Result Interpreter|NDL|O|Y|200
34|Technician|NDL|O|Y|200
35|Transcriptionist|NDL|O|Y|200
36|Scheduled Date/Time|TS|O||26
37|Number of Sample Containers|NM|O||4
38|Transport Logistics of Collected Sample|CE|O|Y|250
39|Collector's Comment|CE|O|Y|250
40|Transport Arrangement Responsibility|CE|O||250
41|Transport Arranged|ID|O||30|0224
42|Escort Required|ID|O||1|0225
43|Planned Patient Transport Comment|CE|O|Y|250
44|Procedure Code|CE|O||250|0088
45|Procedure Code Modifier|CE|O|Y|250|0340
46|Placer Supplemental Service Information|CE|O|Y|250|0411
47|Filler Supplemental Service Information|CE|O|Y|250|0411
48|Medically Necessary Duplicate Procedure Reason|CWE|C||250|0476
49|Result Handling|IS|O||2|0507
50|Parent Universal Service Identifier|CWE|O||250
""")

_segment("OBX", "Observation/Result", """
1|Set ID - OBX|SI|O||4
2|Value Type|ID|C||2|0125
3|Observation Identifier|CE|R||250
4|Observation Sub-ID|ST|C||20
5|Observation Value|varies|C|Y|99999
6|Units|CE|O||250
7|References Range|ST|O||60
8|Abnormal Flags|IS|O|Y|5|0078
9|Probability|NM|O||5
10|Nature of Abnormal Test|ID|O|Y|2|0080
11|Observation Result Status|ID|R||1|0085
12|Effective Date of Reference Range|TS|O||26
13|User Defined Access Checks|ST|O||20
14|Date/Time of the Observation|TS|O||26
15|Producer's ID|CE|O||250
16|Responsible Observer|XCN|O|Y|250
17|Observation Method|CE|O|Y|250
18|Equipment Instance Identifier|EI|O|Y|22
19|Date/Time of the Analysis|TS|O||26
20|Observation Site|CWE|O|Y|705|0163|added in v2.6
21|Observation Instance Identifier|EI|O||427||added in v2.6
22|Mood Code|CNE|C||705|0725|added in v2.6
23|Performing Organization Name|XON|O||567||added in v2.6
24|Performing Organization Address|XAD|O||631||added in v2.6
25|Performing Organization Medical Director|XCN|O||3002||added in v2.6
""")

_segment("NTE", "Notes and Comments", """
1|Set ID - NTE|SI|O||4
2|Source of Comment|ID|O||8|0105
3|Comment|FT|O|Y|65536
4|Comment Type|CE|O||250|0364
""")

_segment("SPM", "Specimen", """
1|Set ID - SPM|SI|O||4
2|Specimen ID|EIP|O||80
3|Specimen Parent IDs|EIP|O|Y|80
4|Specimen Type|CWE|R||250|0487
5|Specimen Type Modifier|CWE|O|Y|250|0541
6|Specimen Additives|CWE|O|Y|250|0371
7|Specimen Collection Method|CWE|O||250|0488
8|Specimen Source Site|CWE|O||250
9|Specimen Source Site Modifier|CWE|O|Y|250|0542
10|Specimen Collection Site|CWE|O||250|0543
11|Specimen Role|CWE|O|Y|250|0369
12|Specimen Collection Amount|CQ|O||20
13|Grouped Specimen Count|NM|C||6
14|Specimen Description|ST|O|Y|250
15|Specimen Handling Code|CWE|O|Y|250|0376
16|Specimen Risk Code|CWE|O|Y|250|0489
17|Specimen Collection Date/Time|DR|O||26
18|Specimen Received Date/Time|TS|O||26
19|Specimen Expiration Date/Time|TS|O||26
20|Specimen Availability|ID|O||1|0136
21|Specimen Reject Reason|CWE|O|Y|250|0490
22|Specimen Quality|CWE|O||250|0491
23|Specimen Appropriateness|CWE|O||250|0492
24|Specimen Condition|CWE|O|Y|250|0493
25|Specimen Current Quantity|CQ|O||20
26|Number of Specimen Containers|NM|O||4
27|Container Type|CWE|O||250
28|Container Condition|CWE|O||250|0544
29|Specimen Child Role|CWE|O||250|0494
""")

_segment("TQ1", "Timing/Quantity", """
1|Set ID - TQ1|SI|O||4
2|Quantity|CQ|O||20
3|Repeat Pattern|RPT|O|Y|540|0335
4|Explicit Time|TM|O|Y|20
5|Relative Time and Units|CQ|O|Y|20
6|Service Duration|CQ|O||20
7|Start Date/Time|TS|O||26
8|End Date/Time|TS|O||26
9|Priority|CWE|O|Y|250|0485
10|Condition Text|TX|O||250
11|Text Instruction|TX|O||250
12|Conjunction|ID|C||10|0427
13|Occurrence Duration|CQ|O||20
14|Total Occurrences|NM|O||10
""")

_segment("RXA", "Pharmacy/Treatment Administration", """
1|Give Sub-ID Counter|NM|R||4
2|Administration Sub-ID Counter|NM|R||4
3|Date/Time Start of Administration|TS|R||26
4|Date/Time End of Administration|TS|R||26
5|Administered Code|CE|R||250|0292
6|Administered Amount|NM|R||20
7|Administered Units|CE|C||250
8|Administered Dosage Form|CE|O||250
9|Administration Notes|CE|O|Y|250
10|Administering Provider|XCN|O|Y|250
11|Administered-at Location|LA2|C||200
12|Administered Per (Time Unit)|ST|C||20
13|Administered Strength|NM|O||20
14|Administered Strength Units|CE|O||250
15|Substance Lot Number|ST|O|Y|20
16|Substance Expiration Date|TS|O|Y|26
17|Substance Manufacturer Name|CE|O|Y|250|0227
18|Substance/Treatment Refusal Reason|CE|O|Y|250
19|Indication|CE|O|Y|250
20|Completion Status|ID|O||2|0322
21|Action Code - RXA|ID|O||2|0323
22|System Entry Date/Time|TS|O||26
23|Administered Drug Strength Volume|NM|O||5
24|Administered Drug Strength Volume Units|CWE|O||250
25|Administered Barcode Identifier|CWE|O||60
26|Pharmacy Order Type|ID|O||1|0480
""")

_segment("RXE", "Pharmacy/Treatment Encoded Order", """
1|Quantity/Timing|TQ|B||200
2|Give Code|CE|R||250|0292
3|Give Amount - Minimum|NM|R||20
4|Give Amount - Maximum|NM|O||20
5|Give Units|CE|R||250
6|Give Dosage Form|CE|O||250
7|Provider's Administration Instructions|CE|O|Y|250
8|Deliver-To Location|LA1|B||200
9|Substitution Status|ID|O||1|0167
10|Dispense Amount|NM|C||20
11|Dispense Units|CE|C||250
12|Number of Refills|NM|O||3
13|Ordering Provider's DEA Number|XCN|C|Y|250
14|Pharmacist/Treatment Supplier's Verifier ID|XCN|O|Y|250
15|Prescription Number|ST|C||20
16|Number of Refills Remaining|NM|C||20
17|Number of Refills/Doses Dispensed|NM|C||20
18|D/T of Most Recent Refill or Dose Dispensed|TS|C||26
19|Total Daily Dose|CQ|C||10
20|Needs Human Review|ID|O||1|0136
21|Pharmacy/Treatment Supplier's Special Dispensing Instructions|CE|O|Y|250
22|Give Per (Time Unit)|ST|C||20
23|Give Rate Amount|ST|O||6
24|Give Rate Units|CE|O||250
25|Give Strength|NM|O||20
26|Give Strength Units|CE|O||250
27|Give Indication|CE|O|Y|250
28|Dispense Package Size|NM|O||20
29|Dispense Package Size Unit|CE|O||250
30|Dispense Package Method|ID|O||2|0321
31|Supplementary Code|CE|O|Y|250
32|Original Order Date/Time|TS|O||26
33|Give Drug Strength Volume|NM|O||5
34|Give Drug Strength Volume Units|CWE|O||250
35|Controlled Substance Schedule|CWE|O||60|0477
36|Formulary Status|ID|O||1|0478
37|Pharmaceutical Substance Alternative|CWE|O|Y|60
38|Pharmacy of Most Recent Fill|CWE|O||250
39|Initial Dispense Amount|NM|O||250
40|Dispensing Pharmacy|CWE|O||250
41|Dispensing Pharmacy Address|XAD|O||250
42|Deliver-to Patient Location|PL|O||80
43|Deliver-to Address|XAD|O||250
44|Pharmacy Order Type|ID|O||1|0480
""")

_segment("RXO", "Pharmacy/Treatment Order", """
1|Requested Give Code|CE|C||250
2|Requested Give Amount - Minimum|NM|C||20
3|Requested Give Amount - Maximum|NM|O||20
4|Requested Give Units|CE|C||250
5|Requested Dosage Form|CE|C||250
6|Provider's Pharmacy/Treatment Instructions|CE|O|Y|250
7|Provider's Administration Instructions|CE|O|Y|250
8|Deliver-To Location|LA1|B||200
9|Allow Substitutions|ID|O||1|0161
10|Requested Dispense Code|CE|O||250
11|Requested Dispense Amount|NM|O||20
12|Requested Dispense Units|CE|O||250
13|Number of Refills|NM|O||3
14|Ordering Provider's DEA Number|XCN|C|Y|250
15|Pharmacist/Treatment Supplier's Verifier ID|XCN|C|Y|250
16|Needs Human Review|ID|O||1|0136
17|Requested Give Per (Time Unit)|ST|C||20
18|Requested Give Strength|NM|O||20
19|Requested Give Strength Units|CE|O||250
20|Indication|CE|O|Y|250
21|Requested Give Rate Amount|ST|O||6
22|Requested Give Rate Units|CE|O||250
23|Total Daily Dose|CQ|O||10
24|Supplementary Code|CE|O|Y|250
25|Requested Drug Strength Volume|NM|O||5
26|Requested Drug Strength Volume Units|CWE|O||250
27|Pharmacy Order Type|ID|O||1|0480
28|Dispensing Interval|NM|O||20
""")

_segment("RXR", "Pharmacy/Treatment Route", """
1|Route|CE|R||250|0162
2|Administration Site|CWE|O||250|0550
3|Administration Device|CE|O||250|0164
4|Administration Method|CWE|O||250|0165
5|Routing Instruction|CE|O||250
6|Administration Site Modifier|CWE|O||250|0495
""")

_segment("RXC", "Pharmacy/Treatment Component Order", """
1|RX Component Type|ID|R||1|0166
2|Component Code|CE|R||250
3|Component Amount|NM|R||20
4|Component Units|CE|R||250
5|Component Strength|NM|O||20
6|Component Strength Units|CE|O||250
7|Supplementary Code|CE|O|Y|250
8|Component Drug Strength Volume|NM|O||5
9|Component Drug Strength Volume Units|CWE|O||250
""")

_segment("SCH", "Scheduling Activity Information", """
1|Placer Appointment ID|EI|C||75
2|Filler Appointment ID|EI|C||75
3|Occurrence Number|NM|C||5
4|Placer Group Number|EI|O||22
5|Schedule ID|CE|O||250
6|Event Reason|CE|R||250
7|Appointment Reason|CE|O||250|0276
8|Appointment Type|CE|O||250|0277
9|Appointment Duration|NM|B||20
10|Appointment Duration Units|CE|B||250
11|Appointment Timing Quantity|TQ|B|Y|200
12|Placer Contact Person|XCN|O|Y|250
13|Placer Contact Phone Number|XTN|O||250
14|Placer Contact Address|XAD|O|Y|250
15|Placer Contact Location|PL|O||80
16|Filler Contact Person|XCN|R|Y|250
17|Filler Contact Phone Number|XTN|O||250
18|Filler Contact Address|XAD|O|Y|250
19|Filler Contact Location|PL|O||80
20|Entered By Person|XCN|R|Y|250
21|Entered By Phone Number|XTN|O|Y|250
22|Entered By Location|PL|O||80
23|Parent Placer Appointment ID|EI|O||75
24|Parent Filler Appointment ID|EI|C||75
25|Filler Status Code|CE|O||250|0278
26|Placer Order Number|EI|C|Y|22
27|Filler Order Number|EI|C|Y|22
""")

_segment("AIS", "Appointment Information", """
1|Set ID - AIS|SI|R||4
2|Segment Action Code|ID|C||3|0206
3|Universal Service Identifier|CE|R||250
4|Start Date/Time|TS|C||26
5|Start Date/Time Offset|NM|C||20
6|Start Date/Time Offset Units|CE|C||250
7|Duration|NM|O||20
8|Duration Units|CE|O||250
9|Allow Substitution Code|IS|C||10|0279
10|Filler Status Code|CE|C||250|0278
11|Placer Supplemental Service Information|CE|O|Y|250|0411
12|Filler Supplemental Service Information|CE|O|Y|250|0411
""")

_segment("AIG", "Appointment Information - General Resource", """
1|Set ID - AIG|SI|R||4
2|Segment Action Code|ID|C||3|0206
3|Resource ID|CE|C||250
4|Resource Type|CE|R||250
5|Resource Group|CE|O|Y|250
6|Resource Quantity|NM|O||5
7|Resource Quantity Units|CE|O||250
8|Start Date/Time|TS|C||26
9|Start Date/Time Offset|NM|C||20
10|Start Date/Time Offset Units|CE|C||250
11|Duration|NM|O||20
12|Duration Units|CE|O||250
13|Allow Substitution Code|IS|C||10|0279
14|Filler Status Code|CE|C||250|0278
""")

_segment("AIL", "Appointment Information - Location Resource", """
1|Set ID - AIL|SI|R||4
2|Segment Action Code|ID|C||3|0206
3|Location Resource ID|PL|C|Y|80
4|Location Type-AIL|CE|R||250|0305
5|Location Group|CE|O||250
6|Start Date/Time|TS|C||26
7|Start Date/Time Offset|NM|C||20
8|Start Date/Time Offset Units|CE|C||250
9|Duration|NM|O||20
10|Duration Units|CE|O||250
11|Allow Substitution Code|IS|C||10|0279
12|Filler Status Code|CE|C||250|0278
""")

_segment("AIP", "Appointment Information - Personnel Resource", """
1|Set ID - AIP|SI|R||4
2|Segment Action Code|ID|C||3|0206
3|Personnel Resource ID|XCN|C|Y|250
4|Resource Type|CE|R||250|0182
5|Resource Group|CE|O||250
6|Start Date/Time|TS|C||26
7|Start Date/Time Offset|NM|C||20
8|Start Date/Time Offset Units|CE|C||250
9|Duration|NM|O||20
10|Duration Units|CE|O||250
11|Allow Substitution Code|IS|C||10|0279
12|Filler Status Code|CE|C||250|0278
""")

_segment("TXA", "Transcription Document Header", """
1|Set ID - TXA|SI|R||4
2|Document Type|IS|R||30|0270
3|Document Content Presentation|ID|C||2|0191
4|Activity Date/Time|TS|O||26
5|Primary Activity Provider Code/Name|XCN|C|Y|250
6|Origination Date/Time|TS|O||26
7|Transcription Date/Time|TS|C||26
8|Edit Date/Time|TS|O|Y|26
9|Originator Code/Name|XCN|O|Y|250
10|Assigned Document Authenticator|XCN|O|Y|250
11|Transcriptionist Code/Name|XCN|C|Y|250
12|Unique Document Number|EI|R||30
13|Parent Document Number|EI|C||30
14|Placer Order Number|EI|O|Y|22
15|Filler Order Number|EI|O||22
16|Unique Document File Name|ST|O||30
17|Document Completion Status|ID|R||2|0271
18|Document Confidentiality Status|ID|O||2|0272
19|Document Availability Status|ID|O||2|0273
20|Document Storage Status|ID|O||2|0275
21|Document Change Reason|ST|C||30
22|Authentication Person, Time Stamp|PPN|C|Y|250
23|Distributed Copies (Code and Name of Recipients)|XCN|O|Y|250
""")

_segment("FT1", "Financial Transaction", """
1|Set ID - FT1|SI|O||4
2|Transaction ID|ST|O||12
3|Transaction Batch ID|ST|O||10
4|Transaction Date|DR|R||53
5|Transaction Posting Date|TS|O||26
6|Transaction Type|IS|R||8|0017
7|Transaction Code|CE|R||250|0132
8|Transaction Description|ST|B||40
9|Transaction Description - Alt|ST|B||40
10|Transaction Quantity|NM|O||6
11|Transaction Amount - Extended|CP|O||12
12|Transaction Amount - Unit|CP|O||12
13|Department Code|CE|O||250|0049
14|Insurance Plan ID|CE|O||250|0072
15|Insurance Amount|CP|O||12
16|Assigned Patient Location|PL|O||80
17|Fee Schedule|IS|O||1|0024
18|Patient Type|IS|O||2|0018
19|Diagnosis Code - FT1|CE|O|Y|250|0051
20|Performed By Code|XCN|O|Y|250|0084
21|Ordered By Code|XCN|O|Y|250
22|Unit Cost|CP|O||12
23|Filler Order Number|EI|O||427
24|Entered By Code|XCN|O|Y|250
25|Procedure Code|CE|O||250|0088
26|Procedure Code Modifier|CE|O|Y|250|0340
27|Advanced Beneficiary Notice Code|CE|O||250|0339
28|Medically Necessary Duplicate Procedure Reason|CWE|O||250|0476
29|NDC Code|CNE|O||250|0549
30|Payment Reference ID|CX|O||250
31|Transaction Reference Key|SI|O|Y|4
""")

_segment("MSA", "Message Acknowledgment", """
1|Acknowledgment Code|ID|R||2|0008
2|Message Control ID|ST|R||20
3|Text Message|ST|B||80
4|Expected Sequence Number|NM|O||15
5|Delayed Acknowledgment Type|ID|B||1
6|Error Condition|CE|B||250|0357
""")

_segment("ERR", "Error", """
1|Error Code and Location|ELD|B|Y|493
2|Error Location|ERL|O|Y|18
3|HL7 Error Code|CWE|R||705|0357
4|Severity|ID|R||2|0516
5|Application Error Code|CWE|O||705|0533
6|Application Error Parameter|ST|O|10|80
7|Diagnostic Information|TX|O||2048
8|User Message|TX|O||250
9|Inform Person Indicator|IS|O|Y|20|0517
10|Override Type|CWE|O||705|0518
11|Override Reason Code|CWE|O|Y|705|0519
12|Help Desk Contact Point|XTN|O|Y|652
""")

_segment("QRD", "Original-Style Query Definition", """
1|Query Date/Time|TS|R||26
2|Query Format Code|ID|R||1|0106
3|Query Priority|ID|R||1|0091
4|Query ID|ST|R||10
5|Deferred Response Type|ID|O||1|0107
6|Deferred Response Date/Time|TS|O||26
7|Quantity Limited Request|CQ|R||10|0126
8|Who Subject Filter|XCN|R|Y|250
9|What Subject Filter|CE|R|Y|250|0048
10|What Department Data Code|CE|R|Y|250
11|What Data Code Value Qual.|VR|O|Y|20
12|Query Results Level|ID|O||1|0108
""")

_segment("QPD", "Query Parameter Definition", """
1|Message Query Name|CE|R||250|0471
2|Query Tag|ST|C||32
3|User Parameters (in successive fields)|varies|O||256
""")

_segment("QAK", "Query Acknowledgment", """
1|Query Tag|ST|C||32
2|Query Response Status|ID|O||2|0208
3|Message Query Name|CE|O||250|0471
4|Hit Count|NM|O||10
5|This Payload|NM|O||10
6|Hits Remaining|NM|O||10
""")

_segment("RCP", "Response Control Parameter", """
1|Query Priority|ID|O||1|0091
2|Quantity Limited Request|CQ|O||10|0126
3|Response Modality|CE|O||250|0394
4|Execution and Delivery Time|TS|C||26
5|Modify Indicator|ID|O||1|0395
6|Sort-by Field|SRT|O|Y|512
7|Segment Group Inclusion|ID|O|Y|256|0391
""")

_segment("DSC", "Continuation Pointer", """
1|Continuation Pointer|ST|O||180
2|Continuation Style|ID|O||1|0398
""")

_segment("FHS", "File Header", """
1|File Field Separator|ST|R||1
2|File Encoding Characters|ST|R||4
3|File Sending Application|HD|O||227
4|File Sending Facility|HD|O||227
5|File Receiving Application|HD|O||227
6|File Receiving Facility|HD|O||227
7|File Creation Date/Time|TS|O||26
8|File Security|ST|O||40
9|File Name/ID|ST|O||20
10|File Header Comment|ST|O||80
11|File Control ID|ST|O||20
12|Reference File Control ID|ST|O||20
""")

_segment("FTS", "File Trailer", """
1|File Batch Count|NM|O||10
2|File Trailer Comment|ST|O||80
""")

_segment("BHS", "Batch Header", """
1|Batch Field Separator|ST|R||1
2|Batch Encoding Characters|ST|R||4
3|Batch Sending Application|HD|O||227
4|Batch Sending Facility|HD|O||227
5|Batch Receiving Application|HD|O||227
6|Batch Receiving Facility|HD|O||227
7|Batch Creation Date/Time|TS|O||26
8|Batch Security|ST|O||40
9|Batch Name/ID/Type|ST|O||20
10|Batch Comment|ST|O||80
11|Batch Control ID|ST|O||20
12|Reference Batch Control ID|ST|O||20
""")

_segment("BTS", "Batch Trailer", """
1|Batch Message Count|ST|O||10
2|Batch Comment|ST|O||80
3|Batch Totals|NM|O|Y|100
""")


# ----------------------------------------------------------------------
# Lookups


def segment(name):
    """The SegmentDef for a segment name; never fails."""
    found = _SEGMENTS.get(name)
    if found:
        return found
    if name.startswith("Z"):
        return SegmentDef(name, "Z-segment (site defined)", [], False)
    return SegmentDef(name, "Unknown segment (not in the v%s dictionary)" % VERSION, [], False)


def field(segment_name, number):
    """The FieldDef for a field such as ("PV1", 7); never fails."""
    seg = segment(segment_name)
    if 1 <= number <= len(seg.fields):
        return seg.fields[number - 1]
    if segment_name.startswith("Z"):
        name = "Site-defined field"
    elif seg.known:
        name = "Not defined in v%s" % VERSION
    else:
        name = "Field %d" % number
    return FieldDef(segment_name, number, name, "", "", None, None, None, None, False)


def datatype(code):
    """The TypeDef for a data type code such as "XCN"; never fails."""
    found = _TYPES.get(code)
    if found:
        return found
    if not code:
        return TypeDef("", "Unknown data type", [], False)
    return TypeDef(code, "Unknown data type", [], False)


def type_label(code):
    """"XCN" -> "XCN — Extended Composite ID Number and Name for Persons"."""
    if not code:
        return "Unknown"
    t = datatype(code)
    return "%s — %s" % (code, t.name)


def is_composite(code):
    return bool(datatype(code).components)


def component(type_code, number):
    """The ComponentDef for component `number` of a data type; never fails."""
    comps = datatype(type_code).components
    if 1 <= number <= len(comps):
        return comps[number - 1]
    return ComponentDef(number, "Component %d" % number, "", None, False)


def segment_names():
    return sorted(_SEGMENTS)


def type_codes():
    return sorted(_TYPES)


def lookup(designation):
    """Describes a designation such as "PID-3", "ORC-4.1" or "PID-3.4.2"
    (dashes also work: "PV1-3-1"). Returns a dict with name, type,
    optionality, repeat, length and table, or None if it can't be read."""
    text = designation.strip().upper()
    if len(text) < 3:
        return None
    seg_name, _, rest = text.partition("-")
    if not rest:
        seg = segment(seg_name)
        return {"name": seg.description, "type": "", "optionality": "", "repeat": None,
                "length": None, "table": None}
    rest = rest.replace("-", ".")
    try:
        numbers = [int(p.split("[")[0]) for p in rest.split(".") if p]
    except ValueError:
        return None
    f = field(seg_name, numbers[0])
    result = {"name": f.name, "type": f.type, "optionality": f.optionality,
              "repeat": f.repeat, "length": f.length, "table": f.table}
    type_code = f.type
    for n in numbers[1:]:
        c = component(type_code, n)
        result.update(name=c.name, type=c.type, table=c.table)
        type_code = c.type
    return result
