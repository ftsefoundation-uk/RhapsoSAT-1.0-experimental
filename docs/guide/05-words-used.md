# 5. Words used

[Guide index](README.md) · previous: [Checking answers](04-checking-answers.md) · next: [Going deeper](06-going-deeper.md)

| Word | Meaning here |
|---|---|
| **Text** | The unknown string of characters the puzzle is about. |
| **Position** | One place in the text. Reports count from 1; certificates and the compact output count from 0. |
| **Rule** | One statement about the text: starts with, ends with, or first k = last k. |
| **Border** | The technical name for "first k = last k": a word that is both a beginning and an end of the text. |
| **Shift** (or **period**) | The fixed distance between positions tied by a border rule. For a text of length n and a border of length k it is n − k. |
| **Tie** (or **link**) | Two positions that one rule forces to hold the same letter. |
| **Group** | All positions connected by ties. They must all hold the same letter. |
| **Open choice** (or **free group**) | A group that no rule gives a letter to. |
| **Pattern** | The description of every solution: forced letters, and numbered open choices. |
| **Contradiction** | The rules cannot all hold; no text exists. |
| **Explanation** (or **chain**) | A list of ties leading from one position to another, showing they must be equal. |
| **Reduction** | Replacing a puzzle by a shorter puzzle of the same kind with the same answer. |
| **Certificate** | A file with an answer and the evidence for it, for a separate checker. |
| **Generator** | One of the few border rules that, alone, force everything all the border rules force. |
