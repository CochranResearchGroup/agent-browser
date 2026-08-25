#!/usr/bin/env python3
"""Repository-owned, network-free X11 fixture for Plan 0131 acceptance."""

import argparse
import tkinter as tk


FIXED_TEXT = "fixture-ready"
TARGET = "#207ad6"
DECOY = "#2179d5"
SUCCESS = "#2ea043"


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--mode",
        choices=[
            "success",
            "ambiguity",
            "focus-loss",
            "geometry-drift",
            "partial-effect",
            "verification-failure",
        ],
        default="success",
    )
    return parser.parse_args()


class Fixture:
    def __init__(self, mode):
        self.mode = mode
        self.armed = False
        self.typed = ""
        self.root = tk.Tk(className="AgentBrowserControlledFixture")
        self.root.title("Agent Browser Controlled X11 Fixture")
        self.root.geometry("800x600+40+40")
        self.root.resizable(False, False)
        self.canvas = tk.Canvas(
            self.root, width=800, height=600, bg="#f6f8fa", highlightthickness=0
        )
        self.canvas.pack(fill="both", expand=True)
        self.canvas.create_text(
            400,
            42,
            text="Controlled X11 foundation fixture",
            fill="#24292f",
            font=("sans", 18),
        )
        self.canvas.create_text(
            400,
            72,
            text="focus epoch 1 | geometry epoch 1",
            fill="#57606a",
            font=("sans", 11),
        )
        self.canvas.create_rectangle(
            160, 120, 255, 167, fill=TARGET, outline=TARGET, tags=("target",)
        )
        self.canvas.create_text(
            208, 190, text="TARGET", fill="#24292f", tags=("target-label",)
        )
        self.canvas.create_rectangle(
            320, 120, 415, 167, fill=DECOY, outline=DECOY, tags=("decoy",)
        )
        self.canvas.create_text(
            368, 144, text="DECOY", fill="white", tags=("decoy",)
        )
        if mode == "ambiguity":
            self.canvas.create_rectangle(
                480, 120, 575, 167, fill=TARGET, outline=TARGET
            )
        self.canvas.create_rectangle(
            160, 220, 575, 265, fill="white", outline="#8c959f"
        )
        self.text_item = self.canvas.create_text(
            176, 242, text="", anchor="w", fill="#24292f", font=("monospace", 14)
        )
        self.canvas.create_text(
            160,
            292,
            text="after state: pending",
            anchor="w",
            fill="#57606a",
            tags=("state",),
        )
        self.canvas.tag_bind("target", "<Button-1>", self.on_target)
        self.root.bind("<KeyPress>", self.on_key)

    def on_target(self, _event):
        self.armed = True
        self.root.focus_force()
        if self.mode == "focus-loss":
            self.root.after(1, self.root.iconify)
        if self.mode == "geometry-drift":
            self.root.geometry("820x620+40+40")

    def on_key(self, event):
        if not self.armed or not event.char:
            return
        if self.mode == "partial-effect" and len(self.typed) >= 3:
            return
        self.typed += event.char
        self.canvas.itemconfigure(self.text_item, text=self.typed)
        if self.typed == FIXED_TEXT and self.mode != "verification-failure":
            self.canvas.delete("state")
            self.canvas.create_rectangle(
                160,
                320,
                415,
                367,
                fill=SUCCESS,
                outline=SUCCESS,
                tags=("state",),
            )
            self.canvas.create_text(
                288, 344, text="VERIFIED", fill="white", tags=("state",)
            )

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    Fixture(parse_args().mode).run()
