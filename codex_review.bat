@echo off
echo Starting CODEX review...
codex exec "请你基于不信任原则，独立审查以下内容：

1. 审查 SPEC/DATA_MODEL.md 中配置同步策略的描述
2. 审查实际代码实现是否完全符合 SPEC 要求
3. 重点检查：
   - push/pull 命令在没有配置名时是否使用 'default'
   - 是否有云端同名配置检测机制
   - 是否有覆盖确认流程
   - 用户选择取消时是否正确终止上传

请提供详细的审查报告，包括：
- SPEC 与代码的一致性分析
- 实现细节的完整性检查
- 任何发现的缺陷或遗漏

项目位置: E:/code/rust/agentic-warden"
pause