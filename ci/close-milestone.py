#!/usr/bin/env python3
import argparse
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("milestone")

args = parser.parse_args()

MILESTONE = args.milestone


def runcmd(command: str, json: bool = False) -> any:
    print("$", command)
    proc = subprocess.run(command, shell=True, stdout=subprocess.PIPE, stderr=None, check=True, text=True)
    if not json:
        return proc.stdout
    else:
        import json
        return json.loads(proc.stdout)


meta = runcmd("gh repo view --json 'name,owner,nameWithOwner'", json=True)
print(meta)

# https://github.com/orgs/software-mansion/projects/4
PROJECT_ID = 4

issues: list[any] = runcmd(f"gh issue list --json 'id,number,title' --milestone '{MILESTONE}' --state closed",
                           json=True)

numbers_to_archive = set((int(i["number"]) for i in issues))
print(len(issues), "issues:", numbers_to_archive)

items: dict[any] = runcmd(
    f"gh projects item-list '{PROJECT_ID}' --org '{meta['owner']['login']}' --format json --limit all", json=True)

items = items["items"]

items_to_archive = [
    item["id"]
    for item in items
    if item['content'].get("repository") == meta["nameWithOwner"] and
       item['content'].get("number") in numbers_to_archive
]

print(len(items_to_archive), "project items to archive")

for issue_id in items_to_archive:
    output = runcmd(f"gh projects item-archive '{PROJECT_ID}' --org '{meta['owner']['login']}' --id '{issue_id}'")
    print(output)

print()
print("Don't forget to Close the milestone manually.")
print("There is not easy to use API for this 🙁")
