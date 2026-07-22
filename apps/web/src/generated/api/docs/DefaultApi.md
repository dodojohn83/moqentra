# DefaultApi

All URIs are relative to *http://localhost*

| Method | HTTP request | Description |
|------------- | ------------- | -------------|
| [**activateAnnotationProject**](DefaultApi.md#activateannotationproject) | **POST** /v1/annotation-projects/{id}/activate | Activate annotation project |
| [**admitTrainingJob**](DefaultApi.md#admittrainingjob) | **POST** /v1/training-jobs/{id}/admit | Admit a training job |
| [**cancelTrainingJob**](DefaultApi.md#canceltrainingjob) | **POST** /v1/training-jobs/{id}/cancel | Cancel a training job |
| [**compileApplication**](DefaultApi.md#compileapplication) | **POST** /v1/applications:compile | Compile an application graph |
| [**createAnnotationProject**](DefaultApi.md#createannotationprojectoperation) | **POST** /v1/annotation-projects | Create annotation project |
| [**createDataset**](DefaultApi.md#createdatasetoperation) | **POST** /v1/datasets | Create dataset |
| [**createDatasetVersion**](DefaultApi.md#createdatasetversionoperation) | **POST** /v1/dataset-versions | Create dataset version |
| [**createExperiment**](DefaultApi.md#createexperimentoperation) | **POST** /v1/experiments | Create experiment |
| [**createModel**](DefaultApi.md#createmodeloperation) | **POST** /v1/models | Create model family |
| [**createTrainingJob**](DefaultApi.md#createtrainingjoboperation) | **POST** /v1/training-jobs | Create training job |
| [**getDataset**](DefaultApi.md#getdataset) | **GET** /v1/datasets/{id} | Get dataset by id |
| [**getHealth**](DefaultApi.md#gethealth) | **GET** /healthz | Liveness probe |
| [**getReady**](DefaultApi.md#getready) | **GET** /readyz | Readiness probe |
| [**listDatasets**](DefaultApi.md#listdatasets) | **GET** /v1/datasets | List datasets for tenant |
| [**listExperiments**](DefaultApi.md#listexperiments) | **GET** /v1/experiments | List experiments |
| [**listModels**](DefaultApi.md#listmodels) | **GET** /v1/models | List models |
| [**listOutboxEvents**](DefaultApi.md#listoutboxevents) | **GET** /v1/outbox | List outbox events |
| [**listTrainingJobs**](DefaultApi.md#listtrainingjobs) | **GET** /v1/training-jobs | List training jobs |
| [**whoAmI**](DefaultApi.md#whoami) | **GET** /v1/whoami | Resolve authenticated principal |



## activateAnnotationProject

> AnnotationProjectResponse activateAnnotationProject(xTenantId, id, authorization)

Activate annotation project

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ActivateAnnotationProjectRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies ActivateAnnotationProjectRequest;

  try {
    const data = await api.activateAnnotationProject(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **id** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**AnnotationProjectResponse**](AnnotationProjectResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Activated |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## admitTrainingJob

> TrainingJobResponse admitTrainingJob(xTenantId, id, authorization)

Admit a training job

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { AdmitTrainingJobRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies AdmitTrainingJobRequest;

  try {
    const data = await api.admitTrainingJob(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **id** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TrainingJobResponse**](TrainingJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Job admitted |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## cancelTrainingJob

> TrainingJobResponse cancelTrainingJob(xTenantId, id, authorization)

Cancel a training job

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CancelTrainingJobRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies CancelTrainingJobRequest;

  try {
    const data = await api.cancelTrainingJob(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **id** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TrainingJobResponse**](TrainingJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Job cancelled |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## compileApplication

> CompileResponse compileApplication(xTenantId, compileRequest, authorization)

Compile an application graph

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CompileApplicationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CompileRequest
    compileRequest: ...,
    // string (optional)
    authorization: authorization_example,
  } satisfies CompileApplicationRequest;

  try {
    const data = await api.compileApplication(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **compileRequest** | [CompileRequest](CompileRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**CompileResponse**](CompileResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Compiled graph |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## createAnnotationProject

> AnnotationProjectResponse createAnnotationProject(xTenantId, createAnnotationProjectRequest, authorization, idempotencyKey)

Create annotation project

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateAnnotationProjectOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateAnnotationProjectRequest
    createAnnotationProjectRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateAnnotationProjectOperationRequest;

  try {
    const data = await api.createAnnotationProject(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **createAnnotationProjectRequest** | [CreateAnnotationProjectRequest](CreateAnnotationProjectRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**AnnotationProjectResponse**](AnnotationProjectResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## createDataset

> DatasetResponse createDataset(xTenantId, createDatasetRequest, authorization, idempotencyKey)

Create dataset

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateDatasetOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateDatasetRequest
    createDatasetRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateDatasetOperationRequest;

  try {
    const data = await api.createDataset(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **createDatasetRequest** | [CreateDatasetRequest](CreateDatasetRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**DatasetResponse**](DatasetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## createDatasetVersion

> DatasetVersionResponse createDatasetVersion(xTenantId, createDatasetVersionRequest, authorization, idempotencyKey)

Create dataset version

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateDatasetVersionOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateDatasetVersionRequest
    createDatasetVersionRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateDatasetVersionOperationRequest;

  try {
    const data = await api.createDatasetVersion(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **createDatasetVersionRequest** | [CreateDatasetVersionRequest](CreateDatasetVersionRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**DatasetVersionResponse**](DatasetVersionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## createExperiment

> ExperimentResponse createExperiment(xTenantId, createExperimentRequest, authorization, idempotencyKey)

Create experiment

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateExperimentOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateExperimentRequest
    createExperimentRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateExperimentOperationRequest;

  try {
    const data = await api.createExperiment(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **createExperimentRequest** | [CreateExperimentRequest](CreateExperimentRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**ExperimentResponse**](ExperimentResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## createModel

> ModelResponse createModel(xTenantId, createModelRequest, authorization, idempotencyKey)

Create model family

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateModelOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateModelRequest
    createModelRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateModelOperationRequest;

  try {
    const data = await api.createModel(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **createModelRequest** | [CreateModelRequest](CreateModelRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**ModelResponse**](ModelResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## createTrainingJob

> TrainingJobResponse createTrainingJob(xTenantId, createTrainingJobRequest, authorization, idempotencyKey)

Create training job

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateTrainingJobOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateTrainingJobRequest
    createTrainingJobRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateTrainingJobOperationRequest;

  try {
    const data = await api.createTrainingJob(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **createTrainingJobRequest** | [CreateTrainingJobRequest](CreateTrainingJobRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TrainingJobResponse**](TrainingJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## getDataset

> DatasetResponse getDataset(xTenantId, id, authorization)

Get dataset by id

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetDatasetRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies GetDatasetRequest;

  try {
    const data = await api.getDataset(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **id** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**DatasetResponse**](DatasetResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Dataset |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## getHealth

> HealthResponse getHealth()

Liveness probe

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetHealthRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  try {
    const data = await api.getHealth();
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters

This endpoint does not need any parameter.

### Return type

[**HealthResponse**](HealthResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | OK |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## getReady

> ReadyResponse getReady()

Readiness probe

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetReadyRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  try {
    const data = await api.getReady();
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters

This endpoint does not need any parameter.

### Return type

[**ReadyResponse**](ReadyResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Ready |  -  |
| **503** | Not ready |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listDatasets

> Page listDatasets(xTenantId, authorization, limit, offset)

List datasets for tenant

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListDatasetsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string (optional)
    authorization: authorization_example,
    // number (optional)
    limit: 56,
    // number (optional)
    offset: 56,
  } satisfies ListDatasetsRequest;

  try {
    const data = await api.listDatasets(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` |  | [Optional] [Defaults to `20`] |
| **offset** | `number` |  | [Optional] [Defaults to `0`] |

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Dataset list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listExperiments

> Page listExperiments(xTenantId, authorization, limit, offset)

List experiments

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListExperimentsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string (optional)
    authorization: authorization_example,
    // number (optional)
    limit: 56,
    // number (optional)
    offset: 56,
  } satisfies ListExperimentsRequest;

  try {
    const data = await api.listExperiments(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` |  | [Optional] [Defaults to `20`] |
| **offset** | `number` |  | [Optional] [Defaults to `0`] |

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Experiment list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listModels

> Page listModels(xTenantId, authorization, limit, offset)

List models

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListModelsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string (optional)
    authorization: authorization_example,
    // number (optional)
    limit: 56,
    // number (optional)
    offset: 56,
  } satisfies ListModelsRequest;

  try {
    const data = await api.listModels(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` |  | [Optional] [Defaults to `20`] |
| **offset** | `number` |  | [Optional] [Defaults to `0`] |

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Model list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listOutboxEvents

> Page listOutboxEvents(xTenantId, authorization, limit, offset)

List outbox events

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListOutboxEventsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string (optional)
    authorization: authorization_example,
    // number (optional)
    limit: 56,
    // number (optional)
    offset: 56,
  } satisfies ListOutboxEventsRequest;

  try {
    const data = await api.listOutboxEvents(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` |  | [Optional] [Defaults to `20`] |
| **offset** | `number` |  | [Optional] [Defaults to `0`] |

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Outbox event list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listTrainingJobs

> Page listTrainingJobs(xTenantId, authorization, limit, offset)

List training jobs

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListTrainingJobsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string (optional)
    authorization: authorization_example,
    // number (optional)
    limit: 56,
    // number (optional)
    offset: 56,
  } satisfies ListTrainingJobsRequest;

  try {
    const data = await api.listTrainingJobs(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **limit** | `number` |  | [Optional] [Defaults to `20`] |
| **offset** | `number` |  | [Optional] [Defaults to `0`] |

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Job list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## whoAmI

> WhoAmIResponse whoAmI(xTenantId, authorization)

Resolve authenticated principal

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { WhoAmIRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies WhoAmIRequest;

  try {
    const data = await api.whoAmI(body);
    console.log(data);
  } catch (error) {
    console.error(error);
  }
}

// Run the test
example().catch(console.error);
```

### Parameters


| Name | Type | Description  | Notes |
|------------- | ------------- | ------------- | -------------|
| **xTenantId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**WhoAmIResponse**](WhoAmIResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Principal context |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)

