# MCP Package Registry Strategy

**Version**: Research Phase
**Date**: 2025-11-19
**Status**: Under Evaluation
**Related**: [MCP CLI Commands Design](./MCP_CLI_COMMANDS_DESIGN.md)

## Executive Summary

基于对MCPM、Claude Code和MCP生态的研究，本文档评估MCP包注册表集成策略，为Phase 3高级功能(`aiw mcp search`, `aiw mcp install`)提供决策依据。

### Key Findings

1. **现有注册表生态**:
   - Smithery.ai - 功能最完善的中心化注册表
   - GitHub MCP Registry - 官方权威来源(44+ servers)
   - Awesome-MCP-Servers - 社区策划列表
   - NPM Packages - 部分MCP服务器通过npm分发

2. **MCPM实现推测**:
   - 未找到明确的注册表URL公开文档
   - 可能聚合多个数据源
   - 实现细节未开源

3. **建议策略**:
   - **Phase 1-2**: 不实现包注册表功能，专注于核心MCP管理
   - **Phase 3**: 评估Smithery.ai集成可行性
   - **备选方案**: GitHub Registry + 本地索引

---

## Existing Registry Ecosystems

### 1. Smithery.ai

**Overview**: 最成熟的MCP服务器注册表平台

**Features**:
- 🔍 **Semantic Search**: 语义搜索MCP服务器
- 📦 **Package Hosting**: 托管和分发MCP包
- 📊 **Usage Metrics**: 下载量、评分、热度
- ✅ **Verification**: 官方认证和安全审核
- 📝 **Documentation**: 每个包的详细文档
- 🏷️ **Categorization**: 按类别组织(文件系统、网络、AI等)

**API Endpoints** (推测):
```
GET https://smithery.ai/api/servers
GET https://smithery.ai/api/servers?q=<query>
GET https://smithery.ai/api/servers/<package-name>
```

**Example Response**:
```json
{
  "servers": [
    {
      "name": "@modelcontextprotocol/server-filesystem",
      "version": "0.6.2",
      "description": "Filesystem operations for MCP",
      "category": "system",
      "downloads": 15234,
      "verified": true,
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem"],
      "repository": "https://github.com/modelcontextprotocol/servers"
    }
  ]
}
```

**Advantages**:
- ✅ 最完善的搜索和发现功能
- ✅ 中心化管理，数据质量高
- ✅ 活跃维护，社区支持
- ✅ RESTful API易于集成

**Disadvantages**:
- ❌ 第三方服务，可用性依赖外部
- ❌ API稳定性和限流政策未知
- ❌ 可能需要API key或认证

**Integration Effort**: 中等 (2-3天)

---

### 2. GitHub MCP Registry

**Overview**: MCP官方维护的权威服务器列表

**Repository**: https://github.com/modelcontextprotocol/servers

**Features**:
- 📚 **Official Source**: 官方认证的44+服务器
- 🔄 **Version Control**: Git管理，版本追踪完整
- 📖 **Documentation**: 每个服务器有README
- 🏗️ **Well-Structured**: 标准化目录结构
- 🆓 **Free & Open**: 完全开源，无限制

**Structure**:
```
modelcontextprotocol/servers/
├── src/
│   ├── filesystem/          # @modelcontextprotocol/server-filesystem
│   ├── git/                 # @modelcontextprotocol/server-git
│   ├── brave-search/        # @modelcontextprotocol/server-brave-search
│   └── ...
└── README.md
```

**Metadata Example** (从package.json提取):
```json
{
  "name": "@modelcontextprotocol/server-filesystem",
  "version": "0.6.2",
  "description": "MCP server for filesystem operations",
  "keywords": ["mcp", "server", "filesystem"],
  "repository": "https://github.com/modelcontextprotocol/servers",
  "license": "MIT"
}
```

**Access Methods**:
1. **GitHub API**:
   ```bash
   GET https://api.github.com/repos/modelcontextprotocol/servers/contents/src
   ```

2. **Git Clone**:
   ```bash
   git clone https://github.com/modelcontextprotocol/servers.git
   # 解析本地文件系统
   ```

3. **Raw Content**:
   ```bash
   GET https://raw.githubusercontent.com/modelcontextprotocol/servers/main/src/filesystem/package.json
   ```

**Advantages**:
- ✅ 官方权威来源
- ✅ 完全免费，无限流
- ✅ 结构化数据，易于解析
- ✅ 可离线缓存

**Disadvantages**:
- ❌ 仅包含官方服务器，第三方包不全
- ❌ 无语义搜索功能
- ❌ 需要自行实现索引和搜索

**Integration Effort**: 中等 (3-5天)

---

### 3. Awesome-MCP-Servers Lists

**Overview**: 社区策划的MCP服务器列表

**Examples**:
- https://github.com/punkpeye/awesome-mcp-servers
- https://github.com/wong2/awesome-mcp-servers
- Others (多个变体)

**Features**:
- 📋 **Curated Lists**: 人工筛选和推荐
- 🌍 **Community Driven**: 社区贡献和更新
- 📝 **Markdown Format**: 简单的README格式
- 🔗 **External Links**: 链接到各个项目

**Structure** (典型):
```markdown
# Awesome MCP Servers

## Official Servers
- [@modelcontextprotocol/server-filesystem](https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem) - Filesystem operations
- [@modelcontextprotocol/server-git](https://github.com/modelcontextprotocol/servers/tree/main/src/git) - Git operations

## Community Servers
- [mcp-server-docker](https://github.com/example/mcp-server-docker) - Docker container management
- ...
```

**Advantages**:
- ✅ 覆盖面广，包括社区项目
- ✅ 人工筛选，质量有保障
- ✅ 简单易解析(Markdown)

**Disadvantages**:
- ❌ 更新不及时
- ❌ 格式不统一
- ❌ 缺少结构化元数据
- ❌ 无搜索功能

**Integration Effort**: 低 (1-2天)
**Maintenance Effort**: 高 (需要定期同步多个源)

---

### 4. NPM Registry

**Overview**: 部分MCP服务器通过npm分发

**Search**:
```bash
npm search mcp server
npm search @modelcontextprotocol
```

**Features**:
- 📦 **Package Management**: npm生态的一部分
- 🔄 **Version Management**: 完善的版本控制
- 📊 **Download Stats**: npm下载量统计
- 📝 **README**: 每个包有文档

**Advantages**:
- ✅ 成熟的包管理生态
- ✅ 完善的版本和依赖管理
- ✅ 广泛使用，社区熟悉

**Disadvantages**:
- ❌ 不是所有MCP服务器都在npm
- ❌ 搜索结果噪音多
- ❌ 需要区分MCP服务器和其他npm包

**Integration Effort**: 低 (NPM API成熟)

---

## MCPM Implementation Analysis

### Research Findings

**MCPM Tool**: https://github.com/wong2/mcpm

**Commands**:
```bash
mcpm search <query>      # 搜索MCP包
mcpm install <package>   # 安装MCP包
mcpm add <name>          # 添加到配置
mcpm remove <name>       # 从配置移除
mcpm enable <name>       # 启用
mcpm disable <name>      # 禁用
mcpm list               # 列出已安装
mcpm restart            # 重启服务
```

**Registry Source** (未明确文档):
- Documentation没有明确说明注册表URL
- 可能使用以下来源的组合:
  1. Smithery.ai API
  2. GitHub MCP Registry
  3. NPM Registry
  4. 内置索引文件

**Implementation Characteristics**:
- 使用TypeScript实现
- 依赖Claude Desktop配置格式
- 未完全开源实现细节

**Reverse Engineering Attempts**:
```bash
# 尝试追踪网络请求
mcpm search filesystem
# 观察HTTP请求目标
```

**Conclusion**:
- MCPM的注册表策略未公开
- 可能使用多源聚合策略
- 实现细节需要代码审查

---

## Recommended Strategy

### Phase 1-2: No Registry Integration

**Rationale**:
1. **Core First**: 优先实现基础MCP管理功能
2. **User Needs**: 大多数用户只需要管理少量已知MCP服务器
3. **Manual Addition**: `aiw mcp add` 足够满足基本需求
4. **Complexity**: 注册表集成增加维护负担

**User Workflow** (without registry):
```bash
# 用户手动查找MCP服务器
# 从文档或GitHub找到命令
aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user
```

**Benefits**:
- ✅ 简单可靠
- ✅ 无外部依赖
- ✅ 快速交付
- ✅ 100% Claude Code兼容

---

### Phase 3: Smithery.ai Integration (Recommended)

**When**: Phase 1-2完成后，如果有明确用户需求

**Implementation Plan**:

1. **Research Smithery.ai API**:
   - 联系Smithery.ai团队获取API文档
   - 确认API稳定性和限流政策
   - 申请API key(如需要)

2. **Implement Search Command**:
   ```rust
   // src/commands/mcp/search.rs
   pub async fn search(query: &str) -> Result<Vec<PackageInfo>> {
       let client = SmitheryClient::new();
       client.search(query).await
   }
   ```

3. **Implement Install Command**:
   ```rust
   // src/commands/mcp/install.rs
   pub async fn install(package: &str) -> Result<()> {
       // 1. Search for package
       // 2. Confirm installation
       // 3. Add to .mcp.json
       // 4. Install dependencies (npm, etc.)
   }
   ```

4. **Add Caching Layer**:
   - 缓存搜索结果(24小时)
   - 离线模式fallback
   - 定期更新索引

**Dependencies**:
```toml
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1.35", features = ["full"] }
serde = "1.0"
serde_json = "1.0"
```

**Estimated Effort**: 1-2周

**Risk Assessment**:
- ⚠️ API可用性依赖第三方
- ⚠️ 需要错误处理和fallback机制
- ⚠️ 可能需要付费或限流

---

### Alternative: GitHub Registry + Local Index

**When**: Smithery.ai不可行或不稳定时

**Implementation Plan**:

1. **Build Local Index**:
   - Clone GitHub MCP Registry
   - 解析package.json metadata
   - 构建本地SQLite索引

2. **Implement Search**:
   ```bash
   aiw mcp search filesystem
   # 搜索本地索引(全文搜索)
   ```

3. **Implement Install**:
   ```bash
   aiw mcp install @modelcontextprotocol/server-filesystem
   # 从npm或GitHub安装
   ```

4. **Update Command**:
   ```bash
   aiw mcp update-index
   # 定期更新本地索引
   ```

**Index Structure** (SQLite):
```sql
CREATE TABLE packages (
    id INTEGER PRIMARY KEY,
    name TEXT UNIQUE,
    version TEXT,
    description TEXT,
    category TEXT,
    command TEXT,
    args JSON,
    repository TEXT,
    verified BOOLEAN,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE INDEX idx_name ON packages(name);
CREATE INDEX idx_category ON packages(category);
CREATE VIRTUAL TABLE packages_fts USING fts5(name, description);
```

**Advantages**:
- ✅ 完全离线工作
- ✅ 快速搜索(本地数据库)
- ✅ 无外部依赖
- ✅ 完全控制数据源

**Disadvantages**:
- ❌ 需要定期更新索引
- ❌ 不包含实时数据(下载量等)
- ❌ 初始构建时间较长

**Estimated Effort**: 2-3周

---

## Hybrid Approach (Best of Both)

**Strategy**: 结合Smithery.ai和本地索引

**Workflow**:
```bash
aiw mcp search <query>
# 1. Try Smithery.ai API (online)
# 2. Fallback to local index (offline)
# 3. Show results with source indicator
```

**Implementation**:
```rust
pub async fn search(query: &str) -> Result<Vec<PackageInfo>> {
    // Try online first
    match smithery::search(query).await {
        Ok(results) => Ok(results),
        Err(_) => {
            // Fallback to local index
            local_index::search(query)
        }
    }
}
```

**Benefits**:
- ✅ 最佳在线体验
- ✅ 离线可用性
- ✅ 数据源多样性

**Complexity**: 高

---

## Decision Matrix

| Strategy | Complexity | Offline | Completeness | Maintenance | Recommended Phase |
|----------|-----------|---------|--------------|-------------|-------------------|
| No Registry | Low | N/A | N/A | None | Phase 1-2 ✅ |
| Smithery.ai | Medium | ❌ | High | Low | Phase 3 ✅ |
| GitHub Index | Medium-High | ✅ | Medium | Medium | Backup |
| Awesome Lists | Low | ✅ | Low | High | ❌ Not recommended |
| NPM Registry | Medium | ❌ | Low | Low | Supplementary |
| Hybrid | High | ✅ | High | High | Future |

---

## Conclusion

### Recommended Path Forward

1. **Phase 1-2 (v5.3.0 - v5.4.0)**:
   - ✅ **No registry integration**
   - ✅ Focus on core MCP management
   - ✅ Users manually add servers via `aiw mcp add`

2. **Phase 3 (v6.0.0)**:
   - 🔍 **Evaluate user demand** for package search
   - 📊 **Research Smithery.ai API** availability and stability
   - 🚀 **Implement if viable**: `aiw mcp search`, `aiw mcp install`

3. **Fallback Strategy**:
   - If Smithery.ai unavailable: GitHub Registry + Local Index
   - If resources limited: Defer to post-v6.0.0

### Success Criteria for Phase 3

Before implementing registry integration, ensure:
- [ ] Phase 1-2 commands are stable and well-adopted
- [ ] User feedback indicates need for package discovery
- [ ] Smithery.ai API is documented and accessible
- [ ] Development resources available (2-3 weeks)

### Alternative: Documentation-First Approach

Instead of building registry integration, provide:
- 📖 **MCP Server Directory** - Curated list in documentation
- 📝 **Quick Start Templates** - Common server configurations
- 🔗 **External Links** - Point to Smithery.ai, GitHub Registry
- 💡 **Examples** - Copy-paste commands for popular servers

**Example Documentation**:
```markdown
## Popular MCP Servers

### Filesystem Operations
```bash
aiw mcp add filesystem npx -y @modelcontextprotocol/server-filesystem /home/user
```

### Git Operations
```bash
aiw mcp add git uvx mcp-server-git --repository /path/to/repo
```

### Web Search
```bash
aiw mcp add brave-search npx -y @modelcontextprotocol/server-brave-search \
  --env BRAVE_API_KEY=your-key
```
```

This approach:
- ✅ Zero implementation cost
- ✅ Provides value immediately
- ✅ Keeps codebase simple
- ✅ Allows users to discover via external resources

---

## Open Questions

1. **Smithery.ai API Access**:
   - Is there a public API?
   - What are the rate limits?
   - Is authentication required?

2. **User Needs**:
   - Do users actually need package search?
   - Or is manual addition sufficient?
   - What's the usage pattern?

3. **Community Input**:
   - Survey existing users
   - Check GitHub issues/discussions
   - Engage with MCP community

---

## Next Actions

1. **Implement Phase 1 Commands** (priority):
   - Focus on `aiw mcp list`, `add`, `remove`, etc.
   - Defer registry integration

2. **Gather User Feedback**:
   - After Phase 1 release
   - Ask about package discovery needs
   - Collect use cases

3. **Research Smithery.ai**:
   - Contact Smithery.ai team
   - Request API documentation
   - Test API stability

4. **Revisit Strategy**:
   - After 3-6 months of Phase 1-2 usage
   - Based on actual user needs
   - With updated ecosystem knowledge

---

**Document Status**: Final Recommendation
**Last Updated**: 2025-11-19
**Decision**: No registry integration for Phase 1-2, re-evaluate for Phase 3
**Next Review**: After Phase 2 completion (~3 months)
