# Machine Learning Development Standards - CODING-STANDARDS-ML

**Version**: 2.0.0
**Scope**: Machine Learning development roles (Supervised/Unsupervised/Reinforcement Learning/Deep Learning, Framework Agnostic)
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - Model requirements must comply with SPEC definitions
       - Metrics, data, deployment based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing model fully matches → Direct reuse
       - Partial match → Retrain from scratch

Iron Rule 3: Prohibit Incremental Development
       - Prohibit fine-tuning old models for new features
       - Prohibit retaining compatibility features

Iron Rule 4: Context7 Research First
       - Use mature ML frameworks and tools
       - Prohibit implementing common algorithms yourself
```

---

## 📊 Data Management

### Data Quality
- ✅ Exploratory Data Analysis (EDA)
- ✅ Missing Value Detection and Handling
- ✅ Outlier Identification and Handling
- ✅ Data Consistency Verification
- ✅ Data Distribution Check
- ❌ Do Not Blindly Use Raw Data

### Data Preprocessing
- ✅ Standardized Data Cleaning Pipeline
- ✅ Feature Scaling (Normalization/Standardization)
- ✅ Categorical Encoding (One-Hot/Label/Target)
- ✅ Text Preprocessing (Tokenization/Stop Word Removal/Stemming)
- ✅ Image Preprocessing (Resize/Augmentation/Normalization)
- ❌ Avoid Data Leakage (Training Set Information Entering Test Set)

### Data Splitting
- ✅ Train/Validation/Test Set Separation
- ✅ Time Series Data Split by Time
- ✅ Stratified Sampling (Maintain Class Distribution)
- ✅ Cross-Validation (K-Fold/Stratified)
- ❌ Prohibit Using Test Set for Model Selection

---

## 🔧 Feature Engineering

### Feature Construction
- ✅ Domain Knowledge-Driven Features
- ✅ Feature Interactions (Combined Features)
- ✅ Polynomial Features
- ✅ Time Feature Extraction (Day/Week/Month/Season)
- ✅ Aggregated Features (Statistics)
- ❌ Avoid Feature Explosion (Curse of Dimensionality)

### Feature Selection
- ✅ Correlation Analysis
- ✅ Feature Importance Ranking
- ✅ Recursive Feature Elimination (RFE)
- ✅ Regularization (L1/L2)
- ✅ Variance Threshold Filtering
- ❌ Avoid Retaining Redundant Features

### Feature Storage
- ✅ Feature Versioning
- ✅ Feature Reuse (Feature Store)
- ✅ Feature Documentation
- ✅ Automated Feature Pipeline
- ❌ Avoid Feature Inconsistency (Training vs Inference)

---

## 🤖 Model Development

### Model Selection
- ✅ Baseline Model First (Simple Model)
- ✅ Select Model Type Based on Problem
- ✅ Compare Multiple Models
- ✅ Ensemble Learning
- ❌ Do Not Use Complex Models Too Early

### Training Process
- ✅ Set Random Seed (Reproducibility)
- ✅ Early Stopping
- ✅ Learning Rate Scheduling
- ✅ Gradient Clipping (Prevent Gradient Explosion)
- ✅ Reasonable Batch Size Settings
- ❌ Avoid Overfitting Training Set

### Hyperparameter Tuning
- ✅ Grid Search
- ✅ Random Search
- ✅ Bayesian Optimization
- ✅ Hyperparameter Importance Analysis
- ✅ Select Best Hyperparameters on Validation Set
- ❌ Do Not Tune on Test Set

---

## 📈 Model Evaluation

### Evaluation Metrics
- ✅ Select Metrics Based on Business Goals
- ✅ Classification: Accuracy/Precision/Recall/F1/AUC
- ✅ Regression: MAE/MSE/RMSE/R²
- ✅ Ranking: NDCG/MAP
- ✅ Multi-Metric Comprehensive Evaluation
- ❌ Single Metric May Be Misleading

### Model Diagnosis
- ✅ Learning Curve Analysis
- ✅ Confusion Matrix (Classification)
- ✅ Residual Analysis (Regression)
- ✅ Feature Importance
- ✅ Error Sample Analysis
- ❌ Do Not Ignore Model Bias

### Overfitting Prevention
- ✅ Regularization (L1/L2/Dropout)
- ✅ Data Augmentation
- ✅ Early Stopping
- ✅ Cross-Validation
- ✅ Simplify Model Complexity
- ❌ Avoid Perfect Training Performance but Poor Generalization

---

## 🔬 Experiment Management

### Experiment Tracking
- ✅ Record Hyperparameter Configuration
- ✅ Record Training Metrics
- ✅ Record Model Version
- ✅ Record Data Version
- ✅ Use Experiment Tracking Tools (MLflow/Weights & Biases)
- ❌ Do Not Manually Record Experiments

### Reproducibility
- ✅ Fixed Random Seed
- ✅ Environment Configuration Versioning (requirements.txt)
- ✅ Code Version Control (Git)
- ✅ Data Version Control (DVC)
- ✅ Parameterized Training Scripts
- ❌ Avoid Unclear Environment Dependencies

---

## 🚀 Model Deployment

### Model Serialization
- ✅ Standard Format Saving (ONNX/SavedModel)
- ✅ Model Versioning
- ✅ Model Metadata Recording
- ✅ Model Compression (Quantization/Pruning)
- ❌ Avoid Using Incompatible Formats

### Inference Optimization
- ✅ Batch Inference
- ✅ Model Quantization (INT8/FP16)
- ✅ Model Distillation (Teacher-Student)
- ✅ Inference Acceleration (TensorRT/ONNX Runtime)
- ✅ Cache Hot Predictions
- ❌ Avoid Excessive Inference Latency

### Online Service
- ✅ Standardized API Interface
- ✅ Input Validation
- ✅ Timeout and Retry Mechanism
- ✅ Load Balancing
- ✅ A/B Testing
- ❌ Do Not Return Model Output Directly (Needs Post-Processing)

---

## 📊 Monitoring and Maintenance

### Performance Monitoring
- ✅ Monitor Inference Latency
- ✅ Monitor Prediction Accuracy
- ✅ Monitor Resource Usage (CPU/GPU/Memory)
- ✅ Monitor Throughput
- ❌ Do Not Deploy Without Monitoring

### Data Drift Detection
- ✅ Input Distribution Monitoring
- ✅ Feature Drift Detection
- ✅ Concept Drift Detection
- ✅ Automatic Alerting
- ✅ Trigger Model Retraining
- ❌ Avoid Model Staleness

### Model Updates
- ✅ Continuous Learning (Incremental Learning)
- ✅ Periodic Retraining
- ✅ New Version Canary Release
- ✅ Rollback Mechanism
- ❌ Do Not Interrupt Online Service

---

## 🔒 Security and Privacy

### Data Privacy
- ✅ Data Desensitization
- ✅ Differential Privacy
- ✅ Federated Learning (Distributed Training)
- ✅ Access Control
- ❌ Prohibit Leaking Training Data

### Model Security
- ✅ Adversarial Sample Detection
- ✅ Model Watermarking
- ✅ Input Validation (Injection Prevention)
- ✅ Output Filtering
- ❌ Avoid Model Reverse Engineering

### Fairness
- ✅ Detect Model Bias
- ✅ Fairness Metrics Evaluation
- ✅ Data Balancing
- ✅ Bias Mitigation Techniques
- ❌ Do Not Ignore Ethical Issues

---

## 🧪 Testing

### Unit Tests
- ✅ Data Preprocessing Function Tests
- ✅ Feature Engineering Function Tests
- ✅ Model Inference Function Tests
- ✅ Boundary Condition Tests
- ❌ Avoid Insufficient Test Coverage

### Integration Tests
- ✅ End-to-End Pipeline Tests
- ✅ Data Validation Tests
- ✅ Model Loading Tests
- ✅ API Interface Tests
- ❌ Do Not Skip Integration Tests

### Model Tests
- ✅ Invariance Tests (Input Changes, Output Unchanged)
- ✅ Directed Expectation Tests (Specific Input, Expected Output)
- ✅ Performance Benchmark Tests
- ✅ Adversarial Sample Tests
- ❌ Avoid Testing Only Happy Paths

---

## 📋 Machine Learning Development Checklist

- [ ] Data quality validation and preprocessing
- [ ] Proper train/validation/test split
- [ ] Feature engineering and selection
- [ ] Baseline model establishment
- [ ] Hyperparameter tuning (validation set)
- [ ] Model evaluation (multiple metrics)
- [ ] Overfitting check
- [ ] Experiment reproducibility (seed/environment/version)
- [ ] Model deployment and inference optimization
- [ ] Monitoring and data drift detection

---

---

## 🏛️ Advanced ML Architecture (20+ Years Experience)

### MLOps Maturity Model
```
Level 0 - Manual Process:
- Manual training and deployment
- No version control
- No monitoring

Level 1 - ML Pipeline Automation:
- Automated training pipeline
- Automated feature engineering
- Experiment tracking

Level 2 - CI/CD Pipeline:
- Model CI/CD
- Automated testing
- Automated deployment

Level 3 - Complete MLOps:
- Continuous Training (CT)
- Continuous Monitoring (CM)
- Automatic Retraining
- Feature Store
```

### Feature Store Architecture
```
Core Components:
- Feature Registry: Metadata, Lineage, Version
- Offline Storage: Batch Features (Data Lake)
- Online Storage: Real-Time Features (Redis/DynamoDB)
- Feature Service: Low-Latency API

Key Capabilities:
- Training-Inference Consistency
- Time Travel (Point-in-Time)
- Feature Reuse
- Data Quality Validation

Technology Selection:
- Feast: Open-Source, Lightweight
- Tecton: Enterprise-Grade
- SageMaker Feature Store: AWS Integration
```

### Large-Scale Training Architecture
```
Distributed Training:
- Data Parallelism
- Model Parallelism
- Pipeline Parallelism
- Hybrid Parallelism

Large Model Training:
- DeepSpeed ZeRO
- Megatron-LM
- FSDP (Fully Sharded Data Parallel)
- Gradient Checkpointing

Hardware Acceleration:
- GPU Clusters (NVIDIA A100/H100)
- TPU Pods (Google Cloud)
- Mixed Precision Training (AMP)
- Compilation Optimization (TorchScript/XLA)
```

### LLM/Generative AI Architecture
```
RAG (Retrieval-Augmented Generation):
- Vector Databases: Pinecone/Milvus/pgvector
- Embedding Models: OpenAI/Cohere/Local
- Retrieval Strategy: Semantic/Hybrid
- Generation Models: GPT/Claude/Open-Source

Fine-Tuning Strategy:
- Full Fine-Tuning
- LoRA/QLoRA (Parameter-Efficient)
- Prompt Tuning
- RLHF/DPO

Production Deployment:
- vLLM/TGI (Inference Optimization)
- Quantization (INT4/INT8/AWQ/GPTQ)
- Caching (KV Cache/Prompt Cache)
- Load Balancing
```

---

## 🔧 Essential Skills for Senior ML Experts

### Deep Learning Tuning
```
Hyperparameter Strategy:
- Learning Rate: Warmup + Cosine Decay
- Batch Size: Progressive Increase
- Regularization: Dropout + Weight Decay
- Data Augmentation: Automation (AutoAugment)

Training Stability:
- Gradient Clipping
- Gradient Accumulation
- Mixed Precision (FP16/BF16)
- Initialization Strategy (Xavier/He/Orthogonal)

Debugging Techniques:
- Overfit Single Batch Verification
- Learning Rate Range Test
- Gradient Distribution Monitoring
- Activation Value Visualization
```

### Model Evaluation Deep Dive
```
Offline Evaluation:
- Stratified Sampling Evaluation
- Time Split Evaluation
- Cross-Validation Strategy
- Statistical Significance Testing

Online Evaluation:
- A/B Test Design
- Multi-Armed Bandit
- Cross-Effect Validation
- Long-Term Effect Tracking

Business Metrics Alignment:
- Model Metrics → Business Metrics Mapping
- Cost-Sensitive Evaluation
- Profit Curve Analysis
- Threshold Optimization
```

### Inference Optimization Deep Dive
```
Model Compression:
- Quantization: PTQ/QAT
- Pruning: Structured/Unstructured
- Distillation: Teacher-Student
- NAS: Automatic Architecture Search

Runtime Optimization:
- Operator Fusion
- Memory Optimization
- Batching Strategy
- Caching Strategy

Hardware Adaptation:
- TensorRT (NVIDIA)
- ONNX Runtime
- OpenVINO (Intel)
- CoreML/Metal (Apple)
```

### Production-Grade Monitoring
```
Data Monitoring:
- Feature Distribution Drift (KS/PSI)
- Missing Value Monitoring
- Data Latency Monitoring
- Data Quality Score

Model Monitoring:
- Prediction Distribution Change
- Confidence Distribution
- Feature Importance Change
- Performance Metrics Trend

Alert Strategy:
- Tiered Alerting (P0-P3)
- Automatic Retraining Trigger
- Manual Review Trigger
- Rollback Mechanism
```

---

## 🚨 Common Pitfalls for Senior ML Experts

### Data Pitfalls
```
❌ Data Leakage:
- Future information enters training
- Test set contamination
- Correct approach: Strict time split, feature review

❌ Sampling Bias:
- Samples do not represent true distribution
- Improper class imbalance handling
- Correct approach: Stratified sampling, weighted loss

❌ Feature Inconsistency:
- Different training and inference feature computation
- Feature version mismatch
- Correct approach: Feature store, unified pipeline
```

### Model Pitfalls
```
❌ Over-Complexity:
- Use complex models from the start
- No baseline comparison
- Correct approach: Simple baseline first

❌ Metric Optimization Excess:
- Focus only on single metric
- Ignore business impact
- Correct approach: Multi-metric balance, business alignment

❌ Improper Validation:
- Random split time series
- No consideration of data distribution
- Correct approach: Design validation based on business scenario
```

### Production Pitfalls
```
❌ Model Aging:
- No monitoring after deployment
- Data drift not handled
- Correct approach: Continuous monitoring, periodic retraining

❌ Ignoring Inference Performance:
- Focus only on model accuracy
- Latency cannot meet SLA
- Correct approach: Balance performance and accuracy

❌ Missing Rollback Mechanism:
- New model online has issues
- Cannot quickly rollback
- Correct approach: Blue-green deployment, fast rollback
```

---

## 📊 Performance Monitoring Metrics

| Metric | Target | Alert Threshold | Measurement Tool |
|--------|--------|-----------------|------------------|
| Model Accuracy | Business Based | Down 5% | Evaluation Pipeline |
| Inference Latency (P99) | < 100ms | > 500ms | APM |
| Feature Drift (PSI) | < 0.1 | > 0.25 | Monitoring System |
| Prediction Distribution Shift | < 0.1 | > 0.2 | Monitoring System |
| Model Training Time | Scenario Based | > 2x Baseline | MLflow |
| GPU Utilization | > 80% | < 50% | Hardware Monitoring |
| Feature Freshness | < 1 hour | > 24 hours | Feature Store |
| Data Quality Score | > 99% | < 95% | Quality Platform |
| Experiment Success Rate | Team Based | Abnormal Drop | Experiment Platform |
| Model Update Frequency | Business Based | Exceed Threshold | Deployment System |

---

## 📋 Machine Learning Development Checklist (Complete)

### Data Pipeline
- [ ] Data quality validation
- [ ] Proper train/validation/test split
- [ ] No data leakage
- [ ] Feature engineering versioning

### Model Development
- [ ] Baseline model establishment
- [ ] Hyperparameter tuning (validation set)
- [ ] Multi-metric evaluation
- [ ] Reproducibility guarantee

### Production Deployment
- [ ] Inference performance optimization
- [ ] Model version management
- [ ] Blue-green/Canary deployment
- [ ] Rollback mechanism

### Continuous Operations
- [ ] Data drift monitoring
- [ ] Model performance monitoring
- [ ] Automatic retraining trigger
- [ ] Alerts and On-call

---

**Machine Learning Development Principles Summary**:
Data Quality, Feature Engineering, Model Evaluation, Reproducibility, Experiment Tracking, Deployment Optimization, Monitoring Drift, Security and Privacy, Fairness, Continuous Improvement
