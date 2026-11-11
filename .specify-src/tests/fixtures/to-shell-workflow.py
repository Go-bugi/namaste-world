"""실제 namaste workflow.yml의 에이전트 단계(command)를 픽스처 셸 단계로 바꾼다.

단계 구조, 루프 상한, 조건식, gate는 원본 그대로 둔다. 그래서 이 테스트가 통과하면
실제 workflow의 루프가 정해진 횟수에서 끝난다는 뜻이 된다.

    python to-shell-workflow.py <원본 workflow.yml> <출력 파일>
"""

import sys

import yaml


def convert(steps):
    out = []
    for step in steps:
        step = dict(step)
        if "command" in step and step.get("type", "command") == "command":
            step = {
                "id": step["id"],
                "type": "shell",
                "run": f"bash .fixture-bin/agent.sh {step['id']}",
            }
        for key in ("steps", "then", "else"):
            if key in step:
                step[key] = convert(step[key])
        out.append(step)
    return out


def main(src, dst):
    with open(src, encoding="utf-8") as f:
        wf = yaml.safe_load(f)
    wf["workflow"]["id"] = "namaste-fixture"
    wf.pop("requires", None)
    wf["steps"] = convert(wf["steps"])
    with open(dst, "w", encoding="utf-8") as f:
        yaml.safe_dump(wf, f, allow_unicode=True, sort_keys=False)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
