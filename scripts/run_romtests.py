#!/usr/bin/env python3
import argparse
import os
import subprocess
import sys
import xml.etree.ElementTree as ET


def parse_args():
    parser = argparse.ArgumentParser(
        description="Run ROM tests from test_roms.xml using make romtest."
    )
    parser.add_argument(
        "--xml",
        default="external/nes-test-roms/test_roms.xml",
        help="Path to test_roms.xml",
    )
    parser.add_argument(
        "--rom-root",
        default="nes-test-roms",
        help="Root directory for ROM paths",
    )
    parser.add_argument(
        "--buffer",
        type=int,
        default=30,
        help="Extra frames to add to runframes",
    )
    parser.add_argument(
        "--frames",
        type=int,
        default=0,
        help="Override runframes from XML (0 = use XML value)",
    )
    parser.add_argument(
        "--filter",
        default="",
        help="Substring filter applied to ROM filename",
    )
    parser.add_argument(
        "--test",
        default="",
        help="Run a single test by exact filename (relative to rom root)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=0,
        help="Max number of tests to run (0 = no limit)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print commands without executing",
    )
    return parser.parse_args()


def load_tests(xml_path, name_filter, exact_test):
    try:
        tree = ET.parse(xml_path)
    except ET.ParseError as exc:
        print(f"Failed to parse XML: {exc}", file=sys.stderr)
        sys.exit(2)
    root = tree.getroot()

    tests = []
    for test in root.findall("test"):
        filename = test.get("filename")
        runframes = test.get("runframes")
        if not filename or not runframes:
            continue
        if exact_test and filename != exact_test:
            continue
        if name_filter and name_filter not in filename:
            continue
        try:
            frames = int(runframes)
        except ValueError:
            continue
        tests.append((filename, frames))
    return tests


def main():
    args = parse_args()
    tests = load_tests(args.xml, args.filter, args.test)
    if args.limit > 0:
        tests = tests[: args.limit]

    if not tests:
        print("No tests matched.")
        return 1

    total = 0
    passed = 0
    failed = 0

    for filename, frames in tests:
        total += 1
        rom_path = os.path.join(args.rom_root, filename)
        effective_frames = args.frames if args.frames > 0 else frames
        cmd = [
            "make",
            "romtest",
            f"rom={rom_path}",
            f"frames={effective_frames}",
            f"buffer={args.buffer}",
        ]
        print(
            f"[{total}/{len(tests)}] {filename} ({effective_frames} + {args.buffer} frames)"
        )
        if args.dry_run:
            print(" ".join(cmd))
            continue

        result = subprocess.run(cmd)
        if result.returncode == 0:
            passed += 1
        else:
            failed += 1

    print("Summary:")
    print(f"  Total:  {total}")
    print(f"  Passed: {passed}")
    print(f"  Failed: {failed}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
