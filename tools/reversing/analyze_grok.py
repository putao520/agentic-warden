#!/usr/bin/env python3
"""
Grok binary 逆向分析脚本（独立工具，非 AIW 集成）

用途：分析 Grok Build binary，定位上传功能的 call 点（patch 锚点），
产出给 AIW 的 patcher/grok/targets.rs 使用。

工具栈：angr（CFG + 交叉引用）+ capstone（反汇编）+ pyelftools（ELF）

方法论（对抗性逆向，针对 0.2.101+ 隐藏 tracing 字符串的情况）：
1. angr CFGFast 构建控制流图（自动识别所有函数 + 交叉引用）
2. 从功能刚需锚点（GCS bucket URL / ExportConfig::Gcs 枚举 / xai-data-collector 模块）
   的代码引用点出发，沿调用图回溯找 upload 入口函数
3. 在 upload 入口函数体里找 HTTP 发送 call（reqwest/hyper send）
4. 产出：upload call 点的偏移 + prologue 字节模式（供 AIW patch）

AIW 只消费本脚本的产出（锚点），不调用本脚本。本脚本是离线分析工具。

用法：
    python3 tools/reversing/analyze_grok.py [--binary PATH] [--version 0.2.101]
"""

import argparse
import json
import logging
import sys
import time

import angr
from elftools.elf.elffile import ELFFile

log = logging.getLogger("grok-rev")


def load_project(binary_path):
    """加载 binary 为 angr Project（不加载依赖库，加速）"""
    t0 = time.time()
    proj = angr.Project(binary_path, auto_load_libs=False)
    log.info("loaded %s in %.1fs (arch=%s entry=%#x)",
             binary_path, time.time() - t0, proj.arch.name, proj.entry)
    return proj


def build_cfg(proj):
    """CFGFast 静态分析：自动识别函数 + 构建交叉引用图"""
    t0 = time.time()
    cfg = proj.analyses.CFGFast(normalize=True, data_references=True)
    log.info("CFGFast done in %.1fs, %d functions recognized",
             time.time() - t0, len(cfg.kb.functions))
    return cfg


def find_string_vaddr(binary_path, needle):
    """在 .rodata 找字符串的 vaddr（rodata vaddr == file offset for grok PIE）"""
    with open(binary_path, "rb") as f:
        data = f.read()
    idx = data.find(needle if isinstance(needle, bytes) else needle.encode())
    return idx if idx >= 0 else None


def find_code_refs_to_data(cfg, data_vaddr):
    """找 .text 里引用某 data vaddr 的所有代码地址（交叉引用）"""
    refs = []
    # angr CFGFast 的 memory_data 记录数据引用
    for block_addr, block in cfg.kb.blocks.items():
        # 检查 block 是否引用了 data_vaddr（通过 lea/mov immediate）
        # 简化：用 block 的 vex/ir 检查常量
        try:
            for insn in block.disassembly.insns:
                if data_vaddr in [insn.op_str] or hex(data_vaddr) in insn.op_str:
                    refs.append(insn.address)
        except Exception:
            pass
    return refs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default="/home/putao/.grok/downloads/grok-linux-x86_64")
    ap.add_argument("--version", default="unknown")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
    )

    log.info("=== Grok binary 逆向分析 (version=%s) ===", args.version)

    proj = load_project(args.binary)
    cfg = build_cfg(proj)

    # TODO: 从 upload 相关字符串/bucket 名出发，沿调用图找 upload call 点
    # 先列出识别到的函数数 + 几个关键函数
    funcs = list(cfg.kb.functions.values())
    log.info("total functions: %d", len(funcs))

    # 输出元信息（后续步骤填充锚点）
    result = {
        "binary": args.binary,
        "version": args.version,
        "functions": len(funcs),
        "anchors": {},  # 后续：每条上传链路的 call 点偏移
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
