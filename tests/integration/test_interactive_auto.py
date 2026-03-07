#!/usr/bin/env python3
"""AIW 交互模式自动化测试"""

import subprocess
import time
import os
import signal
import sys

GREEN = '\033[0;32m'
RED = '\033[0;31m'
YELLOW = '\033[1;33m'
NC = '\033[0m'

def run_cmd(cmd):
    """运行命令并返回输出"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, 
                              text=True, timeout=5)
        return result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired:
        return "", "TIMEOUT", -1

def test_interactive_mode():
    """测试交互模式"""
    AIW = os.path.expanduser("~/.local/bin/aiw")
    
    print("=" * 50)
    print("AIW 交互模式自动化测试")
    print("=" * 50)
    print(f"AIW: {AIW}")
    print()
    
    # 测试 1: 进程替换检查
    print("测试 1: 进程替换检查")
    print(f"启动: {AIW} claude")
    
    proc = subprocess.Popen(
        [AIW, "claude"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL,
        start_new_session=True  # 关键：创建新会话
    )
    pid = proc.pid
    print(f"AIW PID: {pid}")
    
    time.sleep(2)
    
    # 检查 AIW 进程是否还存在
    try:
        os.kill(pid, 0)  # 检查进程是否存在
        print(f"{RED}✗ FAIL{NC} - AIW 还在运行 (PID {pid})")
        proc.kill()
        return False
    except OSError:
        print(f"{GREEN}✓ PASS{NC} - AIW 已退出 (被 exec 替换)")
    
    # 测试 2: 检查 Claude 进程
    print("\n测试 2: 检查 Claude 进程")
    result = run_cmd("pgrep -claude | grep -v grep | head -3")
    lines = result[0].strip().split('\n') if result[0] else []
    
    claude_pids = []
    for line in lines:
        if line.strip():
            parts = line.split()
            if len(parts) >= 2:
                pid = parts[0]
                try:
                    # 获取 PPID
                    ppid_result = run_cmd(f"ps -o ppid -p {pid} 2>/dev/null")
                    ppid = ppid_result[0].strip() if ppid_result[0] else ""
                    print(f"  Claude PID: {pid}, PPID: {ppid}")
                    
                    if ppid == "1":
                        print(f"{GREEN}  ✓{NC} PPID 是 1 (父进程已退出)")
                    claude_pids.append(pid)
                except:
                    pass
    
    if claude_pids:
        print(f"{GREEN}✓ PASS{NC} - 找到 {len(claude_pids)} 个 Claude 进程")
    else:
        print(f"{YELLOW}⊘{NC} SKIP - 未找到 Claude 进程")
    
    return True

def test_non_interactive():
    """测试非交互模式"""
    print("\n" + "=" * 50)
    print("测试 3: 非交互模式")
    print("=" * 50)
    
    AIW = os.path.expanduser("~/.local/bin/aiw")
    
    print(f"运行: {AIW} claude 'test'")
    stdout, stderr, code = run_cmd(f"{AIW} claude test")
    
    if "ToolSearch unlocked" in stdout:
        print(f"{GREEN}✓ ToolSearch 补丁生效{NC}")
    else:
        print(f"{RED}✗ ToolSearch 补丁未生效{NC}")
    
    if "test" in stdout.lower() or "你好" in stdout:
        print(f"{GREEN}✓ Claude 响应正常{NC}")
    else:
        print(f"{YELLOW}! 输出: {stdout[:100]}...{NC}")
    
    return "ToolSearch unlocked" in stdout

def main():
    print("AIW 自动化测试\n")
    
    # 先检查版本
    AIW = os.path.expanduser("~/.local/bin/aiw")
    result = run_cmd(f"{AIW} --version")
    print(f"版本: {result[0].strip()}")
    
    # 测试非交互模式
    non_interactive_ok = test_non_interactive()
    
    # 测试交互模式
    print()
    interactive_ok = test_interactive_mode()
    
    # 总结
    print("\n" + "=" * 50)
    print("测试总结")
    print("=" * 50)
    print(f"非交互模式: {'✓ PASS' if non_interactive_ok else '✗ FAIL'}")
    print(f"交互模式:   {'✓ PASS' if interactive_ok else '✗ FAIL'}")
    print()
    
    if non_interactive_ok and interactive_ok:
        print(f"{GREEN}✓ 所有测试通过！{NC}")
        return 0
    else:
        print(f"{RED}✗ 有测试失败{NC}")
        return 1

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n测试被中断")
        sys.exit(1)
    except Exception as e:
        print(f"{RED}错误: {e}{NC}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
