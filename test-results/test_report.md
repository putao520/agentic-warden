# Agentic-Warden 测试报告

## 测试配置
- 测试类型: gllm-all
- 并行作业数: 20
- 超时时间: 300s
- 运行时间: Mon Dec  1 12:09:38 AM CST 2025
- Git提交: 9a99ba8

## 测试结果
### gllm_embedding_tests
```
test tests::test_empty_text_embedding ... ok
test tests::test_embedding_vector_properties ... ok
test tests::test_repeated_text_embedding ... ok
test tests::test_embedding_cache_behavior ... ok
test tests::test_unicode_text_embedding ... ok
test tests::test_concurrent_embedding_generation ... ok
test tests::test_special_characters_embedding ... ok
test tests::test_single_character_embedding ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.63s

     Running tests/gllm_embedding_demo.rs (target/debug/deps/gllm_embedding_demo-7d63c21e0bd0be59)

running 3 tests
test test_embedding_consistency ... ok
test test_real_embedding_generation ... ok
test test_multilingual_embeddings ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.91s

```

### gllm_rerank_tests
```
test tests::test_embedding_to_rerank_workflow ... ok
test tests::test_cross_lingual_semantic_reranking ... ok
test tests::test_multi_stage_reranking_with_embeddings ... ok
test tests::test_semantic_search_with_rerank ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.56s

     Running tests/gllm_rerank_performance.rs (target/debug/deps/gllm_rerank_performance-682bdd78c397edcc)

running 7 tests
test tests::test_rerank_memory_usage ... ok
test tests::test_rerank_performance_large_dataset ... ok
test tests::test_rerank_cold_vs_warm_performance ... ok
test tests::test_rerank_performance_medium_dataset ... ok
test tests::test_rerank_performance_small_dataset ... ok
test tests::test_rerank_performance_comparison_top_n ... ok
test tests::test_rerank_throughput_scaling ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 30.72s

```

### gllm_integration_tests
```
     Running tests/gllm_embedding_demo.rs (target/debug/deps/gllm_embedding_demo-7d63c21e0bd0be59)

running 3 tests
test test_embedding_consistency ... ok
test test_real_embedding_generation ... ok
test test_multilingual_embeddings ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.13s

     Running tests/gllm_rerank_embedding_integration.rs (target/debug/deps/gllm_rerank_embedding_integration-186a03c1f898a582)

running 5 tests
test tests::test_rerank_with_dynamic_document_updates ... ok
test tests::test_cross_lingual_semantic_reranking ... ok
test tests::test_embedding_to_rerank_workflow ... ok
test tests::test_multi_stage_reranking_with_embeddings ... ok
test tests::test_semantic_search_with_rerank ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.28s

```

