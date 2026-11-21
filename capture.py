"""Renode capture sequence for Budget Ledger (not yet run).

Run from xous-dev-toolkit:
    python3 scripts/renode_capture.py --app budget --script ../precursor-budget/capture.py

Contract: run(ctl, outdir) -> number of failed steps. The app is already launched.
"""
import os


def run(ctl, outdir) -> int:
    failures = 0

    def shot(name):
        nonlocal failures
        if not ctl.try_screenshot(os.path.join(outdir, name)):
            failures += 1

    shot("01_summary_empty.png")
    ctl.inject_key("i")                # new income, cursor on Amount
    ctl.inject_string("2500")
    shot("02_income_form.png")
    ctl.timed_key("F1", after=1.5)     # save -> list
    ctl.inject_key("e")                # new expense
    ctl.inject_string("12.50")
    ctl.timed_key("Down", after=1.0)   # Category
    ctl.timed_key("Right", after=1.0)  # Housing
    ctl.timed_key("Down", after=1.0)   # Date
    ctl.timed_key("Down", after=1.0)   # Note
    ctl.inject_string("rent share")
    shot("03_expense_form.png")
    ctl.timed_key("F1", after=1.5)
    shot("04_list.png")
    ctl.inject_line("")                # view
    ctl.sleep(1.0)
    shot("05_view.png")
    ctl.timed_key("BackSpace", after=1.0)
    ctl.timed_key("BackSpace", after=1.0)
    shot("06_summary.png")
    ctl.timed_key("Left", after=1.0)   # previous month
    shot("07_prev_month.png")
    ctl.timed_key("F2", after=1.0)
    ctl.timed_key("Right", after=1.0)
    shot("08_settings.png")
    ctl.timed_key("F4", after=1.0)
    return failures
