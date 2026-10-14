"""Tests for the HL7 parser and dictionary. Run in this folder with:

    python3 -m unittest

They need only the standard library; the editor isn't involved.
"""

import os
import unittest

import hl7
import hl7_defs as defs

HERE = os.path.dirname(os.path.abspath(__file__))

MESSAGE = "\r".join([
    "MSH|^~\\&|SENDER|FAC^1.2.3^ISO|RECV|RFAC|20260101120000||ADT^A01^ADT_A01|CTRL1|P|2.5.1",
    "PID|1||123^^^HOSP&1.2.3&ISO^MR~456^^^SSA^SS||Doe^Jane^Q||19800101|F",
    "PV1|1|O|FH.ED^^^FHHAZ|EL|||1811^Brat^Tracy^^^^DO^^NPI^^^^NPI",
    "ORC|NW|P123||GRP9^SYS",
    "NTE|1||Line one\\.br\\A \\T\\ B \\F\\ C \\S\\ D \\R\\ E \\E\\ F \\X41\\ \\Zxx\\",
    "ZXX|alpha|beta^gamma",
]) + "\r"


def check_offsets(test, parsed, node):
    """Every node's slice of the text is its raw value, all the way down."""
    test.assertEqual(parsed.text[node.start:node.end], node.raw, node)
    for child in node.children:
        test.assertTrue(node.start <= child.start <= child.end <= node.end, child)
        check_offsets(test, parsed, child)


class Delimiters(unittest.TestCase):
    def test_defaults(self):
        d = hl7.Delimiters()
        self.assertEqual((d.field, d.component, d.repetition, d.escape, d.subcomponent),
                         ("|", "^", "~", "\\", "&"))
        self.assertIsNone(d.truncation)

    def test_read_from_msh(self):
        p = hl7.parse("MSH#*@!%#APP#FAC\rPID#1##12*X*Y@34*Q\r")
        seg = p.segments[1]
        d = seg.delims
        self.assertEqual((d.field, d.component, d.repetition, d.escape, d.subcomponent),
                         ("#", "*", "@", "!", "%"))
        self.assertEqual(p.segments[0].value(3), "APP")
        f = seg.field(3)
        self.assertEqual(len(f.children), 2)
        self.assertEqual(f.child(1).child(2).raw, "X")
        self.assertEqual(f.child(2).child(2).raw, "Q")

    def test_truncation_character(self):
        p = hl7.parse("MSH|^~\\&#|APP\rNTE|1||a\\P\\b\r")
        self.assertEqual(p.segments[0].delims.truncation, "#")
        self.assertEqual(p.segments[0].value(2), "^~\\&#")
        self.assertEqual(hl7.unescape(p.segments[1].value(3), p.segments[1].delims), "a#b")

    def test_missing_subcomponent_separator(self):
        p = hl7.parse("MSH|^~\\|APP\rPID|1||A&B^C\r")
        self.assertIsNone(p.segments[1].delims.subcomponent)
        comp = p.node((1, 3, 1, 1))
        self.assertEqual(comp.raw, "A&B")
        self.assertEqual(len(comp.children), 1)

    def test_empty_encoding_uses_defaults(self):
        p = hl7.parse("MSH||APP\rPID|1||A^B\r")
        self.assertEqual(p.segments[1].delims.component, "^")


class MshNumbering(unittest.TestCase):
    def setUp(self):
        self.p = hl7.parse(MESSAGE)
        self.msh = self.p.segments[0]

    def test_msh_1_and_2(self):
        self.assertEqual(self.msh.name, "MSH")
        self.assertEqual(self.msh.field(1).raw, "|")
        self.assertEqual(self.msh.field(1).start, 3)
        self.assertEqual(self.msh.field(2).raw, "^~\\&")
        self.assertTrue(self.msh.field(2).atomic)
        # The encoding characters aren't split on themselves.
        self.assertEqual(len(self.msh.field(2).children), 1)
        self.assertEqual(self.p.outline_children(self.msh.field(2)), [])

    def test_msh_fields_after_2(self):
        self.assertEqual(self.msh.value(3), "SENDER")
        self.assertEqual(self.msh.value(4, 1, 2), "1.2.3")
        self.assertEqual(self.msh.value(9), "ADT^A01^ADT_A01")
        self.assertEqual(self.msh.value(10), "CTRL1")
        self.assertEqual(self.msh.value(12), "2.5.1")

    def test_message_properties(self):
        m = self.p.messages[0]
        self.assertEqual(m.type, "ADT^A01")
        self.assertEqual(m.structure, "ADT_A01")
        self.assertEqual(m.version, "2.5.1")
        self.assertEqual(m.control_id, "CTRL1")
        self.assertEqual(len(m.segments), 6)

    def test_other_segments_count_from_1(self):
        pv1 = self.p.segments[2]
        self.assertEqual(pv1.value(2), "O")
        self.assertEqual(pv1.value(3), "FH.ED^^^FHHAZ")


class Offsets(unittest.TestCase):
    def setUp(self):
        self.p = hl7.parse(MESSAGE)

    def test_every_node(self):
        for seg in self.p.segments:
            check_offsets(self, self.p, seg)

    def test_field(self):
        f = self.p.node((2, 3))
        self.assertEqual(f.raw, "FH.ED^^^FHHAZ")
        self.assertEqual(MESSAGE[f.start:f.end], "FH.ED^^^FHHAZ")

    def test_repetition(self):
        rep = self.p.node((1, 3, 2))
        self.assertEqual(rep.raw, "456^^^SSA^SS")
        self.assertEqual(MESSAGE[rep.start:rep.end], rep.raw)

    def test_component(self):
        comp = self.p.node((2, 3, 1, 4))
        self.assertEqual(comp.raw, "FHHAZ")
        self.assertEqual(MESSAGE[comp.start:comp.end], "FHHAZ")
        self.assertEqual(self.p.node((2, 7, 1, 13)).raw, "NPI")

    def test_subcomponent(self):
        sub = self.p.node((1, 3, 1, 4, 2))
        self.assertEqual(sub.raw, "1.2.3")
        self.assertEqual(MESSAGE[sub.start:sub.end], "1.2.3")

    def test_code_points(self):
        text = "MSH|^~\\&|A\nPID|1||Zoë😀^Ann\n"
        p = hl7.parse(text)
        comp = p.node((1, 3, 1, 2))
        self.assertEqual(text[comp.start:comp.end], "Ann")

    def test_locate(self):
        p = self.p
        at = MESSAGE.index("FHHAZ") + 2
        self.assertEqual(p.locate(at), (2, 3, 1, 4, 1))
        self.assertEqual(p.visible(p.locate(at)), (2, 3, 1, 4))
        # In an empty component: the field.
        empty = MESSAGE.index("FH.ED^") + 6
        self.assertEqual(p.visible(p.locate(empty)), (2, 3))
        # On the segment name.
        self.assertEqual(p.locate(MESSAGE.index("PV1") + 1), (2,))
        # Just after a separator: the next field. Just before: this one.
        bar = MESSAGE.index("|O|")
        self.assertEqual(p.locate(bar)[:2], (2, 1))
        self.assertEqual(p.locate(bar + 1)[:2], (2, 2))
        # Right after "MSH|": the encoding characters.
        self.assertEqual(p.locate(4), (0, 2, 1, 1, 1))
        # A repetition.
        self.assertEqual(p.visible(p.locate(MESSAGE.index("SSA"))), (1, 3, 2, 4))
        # Past the end of the text.
        self.assertIsNone(p.locate(len(MESSAGE) + 5))

    def test_line_col(self):
        p = hl7.parse("MSH|^~\\&|A\nPID|1||X\n")
        self.assertEqual(p.line_col(0), (1, 1))
        self.assertEqual(p.line_col(p.node((1, 3)).start), (2, 8))
        self.assertEqual(p.segments[1].line, 2)


class LineEndings(unittest.TestCase):
    def check(self, sep):
        text = MESSAGE.replace("\r", sep)
        p = hl7.parse(text)
        self.assertEqual([s.name for s in p.segments], ["MSH", "PID", "PV1", "ORC", "NTE", "ZXX"])
        for seg in p.segments:
            check_offsets(self, p, seg)
            self.assertNotIn("\r", seg.raw)
            self.assertNotIn("\n", seg.raw)
        return p

    def test_cr(self):
        self.check("\r")

    def test_lf(self):
        self.check("\n")

    def test_crlf(self):
        self.check("\r\n")

    def test_blank_lines_and_framing(self):
        p = hl7.parse("\x0bMSH|^~\\&|A\n\n   \nPID|1||X  \n\x1c\r")
        self.assertEqual([s.name for s in p.segments], ["MSH", "PID"])
        self.assertEqual(p.segments[1].value(3), "X")
        check_offsets(self, p, p.segments[0])


class Messages(unittest.TestCase):
    BATCH = "\n".join([
        "FHS|^~\\&|FILEAPP||||||||F1",
        "BHS|^~\\&|BATCHAPP||||||||B1",
        "MSH|^~\\&|A||||||ADT^A08|M1|P|2.5.1",
        "PID|1||1",
        "MSH|^~\\&|A||||||ORU^R01^ORU_R01|M2|P|2.3",
        "PID|1||2",
        "OBX|1|ST|X||hello",
        "BTS|2",
        "FTS|1",
    ])

    def test_messages(self):
        p = hl7.parse(self.BATCH)
        self.assertEqual(len(p.messages), 2)
        self.assertEqual([m.type for m in p.messages], ["ADT^A08", "ORU^R01"])
        self.assertEqual([m.version for m in p.messages], ["2.5.1", "2.3"])
        self.assertEqual([len(m.segments) for m in p.messages], [2, 3])
        self.assertTrue(p.grouped)

    def test_envelope(self):
        p = hl7.parse(self.BATCH)
        names = {s.name: s for s in p.segments}
        for name in ("FHS", "BHS", "BTS", "FTS"):
            self.assertIsNone(names[name].message, name)
        self.assertEqual(names["FHS"].field(1).raw, "|")
        self.assertEqual(names["FHS"].field(2).raw, "^~\\&")
        self.assertEqual(names["FHS"].value(11), "F1")
        self.assertEqual(names["BHS"].value(3), "BATCHAPP")
        self.assertEqual(names["BTS"].value(1), "2")

    def test_single_message_is_not_grouped(self):
        self.assertFalse(hl7.parse(MESSAGE).grouped)

    def test_segments_before_msh(self):
        p = hl7.parse("PID|1||X\nMSH|^~\\&|A||||||ADT^A01|1|P|2.5.1\nPID|1||Y\n")
        self.assertEqual(len(p.messages), 2)
        self.assertIsNone(p.messages[0].header)
        self.assertEqual(p.messages[1].type, "ADT^A01")


class Escapes(unittest.TestCase):
    def test_sequences(self):
        p = hl7.parse(MESSAGE)
        nte = p.segments[4]
        self.assertEqual(hl7.unescape(nte.value(3), nte.delims),
                         "Line one\nA & B | C ^ D ~ E \\ F A \\Zxx\\")

    def test_hex_utf8(self):
        d = hl7.Delimiters()
        self.assertEqual(hl7.unescape("caf\\XC3A9\\", d), "café")
        self.assertEqual(hl7.unescape("\\XE9\\", d), "é")

    def test_formatting(self):
        d = hl7.Delimiters()
        self.assertEqual(hl7.unescape("a\\.sp 2\\b", d), "a\n\nb")
        self.assertEqual(hl7.unescape("a\\.sk 3\\b", d), "a   b")
        self.assertEqual(hl7.unescape("\\H\\bold\\N\\", d), "bold")
        self.assertEqual(hl7.unescape("a\\.in +4\\b", d), "ab")

    def test_unterminated_and_unknown(self):
        d = hl7.Delimiters()
        self.assertEqual(hl7.unescape("a\\F", d), "a\\F")
        self.assertEqual(hl7.unescape("\\Q\\", d), "\\Q\\")
        self.assertEqual(hl7.unescape("\\XZZ\\", d), "\\XZZ\\")

    def test_other_delimiters(self):
        d = hl7.Delimiters("#", "*@!%")
        self.assertEqual(hl7.unescape("a!F!b!S!c!E!", d), "a#b*c!")

    def test_escaped_separators_are_not_split(self):
        p = hl7.parse("MSH|^~\\&|A\nNTE|1||x\\S\\y\\F\\z\n")
        f = p.node((1, 3))
        self.assertEqual(len(f.children[0].children), 1)


class Outline(unittest.TestCase):
    def setUp(self):
        self.p = hl7.parse(MESSAGE)

    def test_hidden_empty_pieces(self):
        pv1 = self.p.segments[2]
        fields = [f.path[1] for f in self.p.outline_children(pv1)]
        self.assertEqual(fields, [1, 2, 3, 4, 7])
        comps = [c.path[3] for c in self.p.outline_children(pv1.field(3))]
        self.assertEqual(comps, [1, 4])
        self.assertEqual(self.p.outline_children(pv1.field(1)), [])

    def test_repetitions(self):
        pid3 = self.p.node((1, 3))
        reps = self.p.outline_children(pid3)
        self.assertEqual([r.path for r in reps], [(1, 3, 1), (1, 3, 2)])
        subs = self.p.outline_children(self.p.node((1, 3, 1, 4)))
        self.assertEqual([s.raw for s in subs], ["HOSP", "1.2.3", "ISO"])

    def test_designations(self):
        p = self.p
        self.assertEqual(hl7.designation(p, (2,)), "PV1")
        self.assertEqual(hl7.designation(p, (2, 7)), "PV1-7")
        self.assertEqual(hl7.designation(p, (2, 7, 1, 1)), "PV1-7.1")
        self.assertEqual(hl7.designation(p, (2, 3, 1, 4), "-"), "PV1-3-4")
        self.assertEqual(hl7.designation(p, (1, 3, 2)), "PID-3[2]")
        self.assertEqual(hl7.designation(p, (1, 3, 1, 4, 1)), "PID-3[1].4.1")
        self.assertEqual(hl7.designation(p, (1, 3, 1, 4, 1), "-"), "PID-3[1]-4-1")
        self.assertEqual(hl7.designation(p, (3, 4, 1, 1)), "ORC-4.1")

    def test_ids(self):
        self.assertEqual(hl7.path_id((2, 7, 1, 13)), "2.7.1.13")
        self.assertEqual(hl7.parse_id("2.7.1.13"), (2, 7, 1, 13))
        self.assertIsNone(hl7.parse_id("m1"))
        self.assertIsNone(hl7.parse_id("2.0"))

    def test_shorten(self):
        self.assertEqual(hl7.shorten("abc", 120), "abc")
        long = hl7.shorten("x" * 300, 120)
        self.assertEqual(len(long), 120)
        self.assertTrue(long.endswith("…"))

    def test_looks_like_hl7(self):
        self.assertTrue(hl7.looks_like_hl7("MSH|^~\\&|"))
        self.assertTrue(hl7.looks_like_hl7("\n  FHS|^~\\&|"))
        self.assertTrue(hl7.looks_like_hl7("\x0bBHS|^~\\&|"))
        self.assertTrue(hl7.looks_like_hl7("anything", "/tmp/a.HL7"))
        self.assertFalse(hl7.looks_like_hl7("Hello MSH|"))
        self.assertFalse(hl7.looks_like_hl7("", "/tmp/a.txt"))


class Dictionary(unittest.TestCase):
    def test_lookups(self):
        self.assertEqual(defs.lookup("ORC-4.1")["name"], "Entity Identifier")
        self.assertEqual(defs.lookup("ORC-4")["type"], "EI")
        self.assertEqual(defs.lookup("PV1-7")["type"], "XCN")
        self.assertEqual(defs.lookup("PV1-3")["name"], "Assigned Patient Location")
        self.assertEqual(defs.lookup("PV1-3-1")["name"], "Point of Care")
        self.assertEqual(defs.lookup("PID-3")["repeat"], 0)
        self.assertEqual(defs.lookup("PID-3.4.2")["name"], "Universal ID")
        self.assertEqual(defs.lookup("PV1-7.2.1")["name"], "Surname")
        self.assertEqual(defs.lookup("MSH-9.2")["name"], "Trigger Event")
        self.assertEqual(defs.lookup("PID-8")["table"], "0001")
        self.assertEqual(defs.lookup("PID-38")["repeat"], 2)

    def test_field(self):
        f = defs.field("PV1", 7)
        self.assertEqual((f.name, f.type, f.optionality, f.repeat, f.length, f.table),
                         ("Attending Doctor", "XCN", "O", 0, 250, "0010"))
        self.assertEqual(defs.field("PID", 5).optionality, "R")
        self.assertEqual(defs.field("MSH", 1).name, "Field Separator")
        self.assertEqual(defs.field("MSH", 2).name, "Encoding Characters")

    def test_fallbacks(self):
        z = defs.segment("ZXX")
        self.assertEqual(z.description, "Z-segment (site defined)")
        self.assertFalse(z.known)
        self.assertEqual(defs.field("ZXX", 3).name, "Site-defined field")
        self.assertFalse(defs.segment("QQQ").known)
        self.assertEqual(defs.field("QQQ", 2).name, "Field 2")
        self.assertIn("Not defined", defs.field("PID", 99).name)
        self.assertEqual(defs.datatype("QQ").name, "Unknown data type")
        self.assertEqual(defs.component("ST", 2).name, "Component 2")
        self.assertEqual(defs.type_label(""), "Unknown")

    def test_texts(self):
        self.assertEqual(defs.optionality_text("R"), "Required (R)")
        self.assertEqual(defs.optionality_text("B"), "Backward compatible (kept for older versions) (B)")
        self.assertEqual(defs.repeat_text(1), "No")
        self.assertEqual(defs.repeat_text(0), "Yes")
        self.assertEqual(defs.repeat_text(2), "Yes, up to 2 times")
        self.assertEqual(defs.type_label("XCN"),
                         "XCN — Extended Composite ID Number and Name for Persons")

    def test_required_segments(self):
        for name in ("MSH EVN PID PD1 NK1 PV1 PV2 MRG AL1 DG1 PR1 GT1 IN1 IN2 ROL ORC OBR OBX "
                     "NTE SPM TQ1 RXA RXE RXO RXR RXC SCH AIS AIG AIL AIP TXA FT1 MSA ERR QRD "
                     "QPD RCP FHS BHS BTS FTS DSC IAM").split():
            seg = defs.segment(name)
            self.assertTrue(seg.known, name)
            self.assertTrue(seg.fields, name)

    def test_every_type_is_defined(self):
        """Every type used by a field or a component has an entry."""
        missing = set()
        for name in defs.segment_names():
            for f in defs.segment(name).fields:
                if not defs.datatype(f.type).known:
                    missing.add("%s-%d %s" % (name, f.number, f.type))
        for code in defs.type_codes():
            for c in defs.datatype(code).components:
                if not defs.datatype(c.type).known:
                    missing.add("%s.%d %s" % (code, c.number, c.type))
        self.assertEqual(missing, set())

    def test_field_values_are_sane(self):
        for name in defs.segment_names():
            for f in defs.segment(name).fields:
                self.assertIn(f.optionality, defs.OPTIONALITY, (name, f.number))
                self.assertTrue(f.repeat is not None and f.repeat >= 0, (name, f.number))
                self.assertTrue(f.table is None or (len(f.table) == 4 and f.table.isdigit()),
                                (name, f.number))


class Sample(unittest.TestCase):
    def test_sample_file(self):
        with open(os.path.join(HERE, "sample.hl7"), encoding="utf-8", newline="") as f:
            text = f.read()
        self.assertTrue(hl7.looks_like_hl7(text))
        p = hl7.parse(text)
        self.assertEqual(len(p.messages), 1)
        self.assertEqual(p.messages[0].type, "ORM^O01")
        self.assertEqual(p.messages[0].version, "2.5.1")
        for seg in p.segments:
            check_offsets(self, p, seg)
            sdef = defs.segment(seg.name)
            if sdef.known:
                self.assertLessEqual(len(seg.children), len(sdef.fields), seg.name)


if __name__ == "__main__":
    unittest.main()
