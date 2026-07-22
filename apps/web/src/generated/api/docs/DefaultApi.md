# DefaultApi

All URIs are relative to *http://localhost*

| Method | HTTP request | Description |
|------------- | ------------- | -------------|
| [**abortUploadSession**](DefaultApi.md#abortuploadsession) | **DELETE** /v1/upload-sessions/{id} | Abort an upload session |
| [**activateAnnotationProject**](DefaultApi.md#activateannotationproject) | **POST** /v1/annotation-projects/{id}/activate | Activate annotation project |
| [**addDatasetVersionAsset**](DefaultApi.md#adddatasetversionasset) | **POST** /v1/dataset-versions/{id}/assets | Add an asset to a draft dataset version |
| [**admitTrainingJob**](DefaultApi.md#admittrainingjob) | **POST** /v1/training-jobs/{id}/admit | Admit a training job |
| [**approveAnnotationTask**](DefaultApi.md#approveannotationtask) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/approve | Approve an annotation task |
| [**assignAnnotationTask**](DefaultApi.md#assignannotationtask) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/assign | Assign/claim an annotation task |
| [**autosaveAnnotation**](DefaultApi.md#autosaveannotation) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/annotations | Autosave an annotation |
| [**cancelImportJob**](DefaultApi.md#cancelimportjob) | **DELETE** /v1/import-jobs/{id} | Cancel an import job |
| [**cancelTrainingJob**](DefaultApi.md#canceltrainingjob) | **POST** /v1/training-jobs/{id}/cancel | Cancel a training job |
| [**compileApplication**](DefaultApi.md#compileapplication) | **POST** /v1/applications:compile | Compile an application graph |
| [**completeUploadSession**](DefaultApi.md#completeuploadsession) | **POST** /v1/upload-sessions/{id}/complete | Complete an upload session |
| [**createAnnotationProject**](DefaultApi.md#createannotationprojectoperation) | **POST** /v1/annotation-projects | Create annotation project |
| [**createAnnotationTasks**](DefaultApi.md#createannotationtasks) | **POST** /v1/annotation-projects/{id}/tasks | Create annotation tasks |
| [**createDataset**](DefaultApi.md#createdatasetoperation) | **POST** /v1/datasets | Create dataset |
| [**createDatasetVersion**](DefaultApi.md#createdatasetversionoperation) | **POST** /v1/dataset-versions | Create dataset version |
| [**createExperiment**](DefaultApi.md#createexperimentoperation) | **POST** /v1/experiments | Create experiment |
| [**createImportJob**](DefaultApi.md#createimportjoboperation) | **POST** /v1/import-jobs | Create an S3/MinIO import job |
| [**createModel**](DefaultApi.md#createmodeloperation) | **POST** /v1/models | Create model family |
| [**createTrainingJob**](DefaultApi.md#createtrainingjoboperation) | **POST** /v1/training-jobs | Create training job |
| [**createUploadSession**](DefaultApi.md#createuploadsessionoperation) | **POST** /v1/upload-sessions | Create a multipart upload session |
| [**generateDatasetVersionSplits**](DefaultApi.md#generatedatasetversionsplits) | **POST** /v1/dataset-versions/{id}/splits | Generate deterministic train/val/test splits |
| [**getAnnotationTask**](DefaultApi.md#getannotationtask) | **GET** /v1/annotation-projects/{id}/tasks/{taskId} | Get annotation task |
| [**getAssetMediaUrl**](DefaultApi.md#getassetmediaurl) | **GET** /v1/assets/{assetId}/media-url | Get short-lived signed media URL |
| [**getDataset**](DefaultApi.md#getdataset) | **GET** /v1/datasets/{id} | Get dataset by id |
| [**getHealth**](DefaultApi.md#gethealth) | **GET** /healthz | Liveness probe |
| [**getImportJob**](DefaultApi.md#getimportjob) | **GET** /v1/import-jobs/{id} | Get import job status |
| [**getReady**](DefaultApi.md#getready) | **GET** /readyz | Readiness probe |
| [**getUploadSession**](DefaultApi.md#getuploadsession) | **GET** /v1/upload-sessions/{id} | Get upload session |
| [**listAnnotationTasks**](DefaultApi.md#listannotationtasks) | **GET** /v1/annotation-projects/{id}/tasks | List annotation tasks |
| [**listAnnotations**](DefaultApi.md#listannotations) | **GET** /v1/annotation-projects/{id}/tasks/{taskId}/annotations | List annotations for a task |
| [**listDatasets**](DefaultApi.md#listdatasets) | **GET** /v1/datasets | List datasets for tenant |
| [**listExperiments**](DefaultApi.md#listexperiments) | **GET** /v1/experiments | List experiments |
| [**listModels**](DefaultApi.md#listmodels) | **GET** /v1/models | List models |
| [**listOutboxEvents**](DefaultApi.md#listoutboxevents) | **GET** /v1/outbox | List outbox events |
| [**listPartUploadUrls**](DefaultApi.md#listpartuploadurls) | **GET** /v1/upload-sessions/{id}/part-urls | List signed URLs for uploading parts |
| [**listTrainingJobs**](DefaultApi.md#listtrainingjobs) | **GET** /v1/training-jobs | List training jobs |
| [**listUploadSessionParts**](DefaultApi.md#listuploadsessionparts) | **GET** /v1/upload-sessions/{id}/parts | List upload session parts |
| [**publishDatasetVersion**](DefaultApi.md#publishdatasetversion) | **POST** /v1/dataset-versions/{id}/publish | Publish a dataset version |
| [**returnAnnotationTask**](DefaultApi.md#returnannotationtask) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/return | Return an annotation task for rework |
| [**startAnnotationTask**](DefaultApi.md#startannotationtask) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/start | Start an assigned annotation task |
| [**submitAnnotationTask**](DefaultApi.md#submitannotationtask) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/submit | Submit annotations for review |
| [**uploadPart**](DefaultApi.md#uploadpart) | **POST** /v1/upload-sessions/{id}/parts/{partNumber} | Upload a part |
| [**whoAmI**](DefaultApi.md#whoami) | **GET** /v1/whoami | Resolve authenticated principal |



## abortUploadSession

> abortUploadSession(xTenantId, id, authorization)

Abort an upload session

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { AbortUploadSessionRequest } from '';

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
  } satisfies AbortUploadSessionRequest;

  try {
    const data = await api.abortUploadSession(body);
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

`void` (Empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **204** | Aborted |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


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


## addDatasetVersionAsset

> DatasetVersionResponse addDatasetVersionAsset(xTenantId, id, addAssetRequest, authorization, idempotencyKey)

Add an asset to a draft dataset version

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { AddDatasetVersionAssetRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // AddAssetRequest
    addAssetRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies AddDatasetVersionAssetRequest;

  try {
    const data = await api.addDatasetVersionAsset(body);
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
| **addAssetRequest** | [AddAssetRequest](AddAssetRequest.md) |  | |
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
| **200** | Asset added |  -  |

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


## approveAnnotationTask

> TaskResponse approveAnnotationTask(xTenantId, id, taskId, authorization)

Approve an annotation task

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ApproveAnnotationTaskRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies ApproveAnnotationTaskRequest;

  try {
    const data = await api.approveAnnotationTask(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Approved task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## assignAnnotationTask

> TaskResponse assignAnnotationTask(xTenantId, id, taskId, assignRequest, authorization)

Assign/claim an annotation task

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { AssignAnnotationTaskRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // AssignRequest
    assignRequest: ...,
    // string (optional)
    authorization: authorization_example,
  } satisfies AssignAnnotationTaskRequest;

  try {
    const data = await api.assignAnnotationTask(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **assignRequest** | [AssignRequest](AssignRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Assigned task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## autosaveAnnotation

> AnnotationResponse autosaveAnnotation(xTenantId, id, taskId, autosaveRequest, authorization)

Autosave an annotation

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { AutosaveAnnotationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // AutosaveRequest
    autosaveRequest: ...,
    // string (optional)
    authorization: authorization_example,
  } satisfies AutosaveAnnotationRequest;

  try {
    const data = await api.autosaveAnnotation(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **autosaveRequest** | [AutosaveRequest](AutosaveRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**AnnotationResponse**](AnnotationResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/json`
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Saved annotation |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## cancelImportJob

> cancelImportJob(xTenantId, id, authorization)

Cancel an import job

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CancelImportJobRequest } from '';

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
  } satisfies CancelImportJobRequest;

  try {
    const data = await api.cancelImportJob(body);
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

`void` (Empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **204** | Cancelled |  -  |

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


## completeUploadSession

> UploadSessionResponse completeUploadSession(xTenantId, id, authorization)

Complete an upload session

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CompleteUploadSessionRequest } from '';

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
  } satisfies CompleteUploadSessionRequest;

  try {
    const data = await api.completeUploadSession(body);
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

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Completed |  -  |

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


## createAnnotationTasks

> Array&lt;TaskResponse&gt; createAnnotationTasks(xTenantId, id, createTasksRequest, authorization)

Create annotation tasks

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateAnnotationTasksRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // CreateTasksRequest
    createTasksRequest: ...,
    // string (optional)
    authorization: authorization_example,
  } satisfies CreateAnnotationTasksRequest;

  try {
    const data = await api.createAnnotationTasks(body);
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
| **createTasksRequest** | [CreateTasksRequest](CreateTasksRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**Array&lt;TaskResponse&gt;**](TaskResponse.md)

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


## createImportJob

> ImportJobResponse createImportJob(xTenantId, createImportJobRequest, authorization, idempotencyKey)

Create an S3/MinIO import job

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateImportJobOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateImportJobRequest
    createImportJobRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateImportJobOperationRequest;

  try {
    const data = await api.createImportJob(body);
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
| **createImportJobRequest** | [CreateImportJobRequest](CreateImportJobRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**ImportJobResponse**](ImportJobResponse.md)

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


## createUploadSession

> UploadSessionResponse createUploadSession(xTenantId, createUploadSessionRequest, authorization, idempotencyKey)

Create a multipart upload session

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { CreateUploadSessionOperationRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // CreateUploadSessionRequest
    createUploadSessionRequest: ...,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    idempotencyKey: idempotencyKey_example,
  } satisfies CreateUploadSessionOperationRequest;

  try {
    const data = await api.createUploadSession(body);
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
| **createUploadSessionRequest** | [CreateUploadSessionRequest](CreateUploadSessionRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **idempotencyKey** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**UploadSessionResponse**](UploadSessionResponse.md)

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


## generateDatasetVersionSplits

> DatasetVersionResponse generateDatasetVersionSplits(xTenantId, id, generateSplitsRequest, authorization)

Generate deterministic train/val/test splits

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GenerateDatasetVersionSplitsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // GenerateSplitsRequest
    generateSplitsRequest: ...,
    // string (optional)
    authorization: authorization_example,
  } satisfies GenerateDatasetVersionSplitsRequest;

  try {
    const data = await api.generateDatasetVersionSplits(body);
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
| **generateSplitsRequest** | [GenerateSplitsRequest](GenerateSplitsRequest.md) |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

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
| **200** | Splits generated |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## getAnnotationTask

> TaskResponse getAnnotationTask(xTenantId, id, taskId, authorization)

Get annotation task

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetAnnotationTaskRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies GetAnnotationTaskRequest;

  try {
    const data = await api.getAnnotationTask(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## getAssetMediaUrl

> MediaUrlResponse getAssetMediaUrl(xTenantId, assetId, authorization)

Get short-lived signed media URL

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetAssetMediaUrlRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    assetId: assetId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies GetAssetMediaUrlRequest;

  try {
    const data = await api.getAssetMediaUrl(body);
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
| **assetId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**MediaUrlResponse**](MediaUrlResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Signed URL |  -  |

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


## getImportJob

> ImportJobResponse getImportJob(xTenantId, id, authorization)

Get import job status

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetImportJobRequest } from '';

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
  } satisfies GetImportJobRequest;

  try {
    const data = await api.getImportJob(body);
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

[**ImportJobResponse**](ImportJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Import job |  -  |

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


## getUploadSession

> UploadSessionResponse getUploadSession(xTenantId, id, authorization)

Get upload session

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { GetUploadSessionRequest } from '';

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
  } satisfies GetUploadSessionRequest;

  try {
    const data = await api.getUploadSession(body);
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

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Session |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listAnnotationTasks

> Array&lt;TaskResponse&gt; listAnnotationTasks(xTenantId, id, authorization)

List annotation tasks

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListAnnotationTasksRequest } from '';

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
  } satisfies ListAnnotationTasksRequest;

  try {
    const data = await api.listAnnotationTasks(body);
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

[**Array&lt;TaskResponse&gt;**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Task list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## listAnnotations

> Array&lt;AnnotationResponse&gt; listAnnotations(xTenantId, id, taskId, authorization)

List annotations for a task

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListAnnotationsRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies ListAnnotationsRequest;

  try {
    const data = await api.listAnnotations(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**Array&lt;AnnotationResponse&gt;**](AnnotationResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Annotation list |  -  |

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


## listPartUploadUrls

> Array&lt;UploadPartUrl&gt; listPartUploadUrls(xTenantId, id, authorization)

List signed URLs for uploading parts

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListPartUploadUrlsRequest } from '';

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
  } satisfies ListPartUploadUrlsRequest;

  try {
    const data = await api.listPartUploadUrls(body);
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

[**Array&lt;UploadPartUrl&gt;**](UploadPartUrl.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Signed part upload URLs |  -  |

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


## listUploadSessionParts

> UploadSessionResponse listUploadSessionParts(xTenantId, id, authorization)

List upload session parts

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ListUploadSessionPartsRequest } from '';

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
  } satisfies ListUploadSessionPartsRequest;

  try {
    const data = await api.listUploadSessionParts(body);
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

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Session |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## publishDatasetVersion

> DatasetVersionResponse publishDatasetVersion(xTenantId, id, authorization)

Publish a dataset version

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { PublishDatasetVersionRequest } from '';

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
  } satisfies PublishDatasetVersionRequest;

  try {
    const data = await api.publishDatasetVersion(body);
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

[**DatasetVersionResponse**](DatasetVersionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Version published |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## returnAnnotationTask

> TaskResponse returnAnnotationTask(xTenantId, id, taskId, authorization)

Return an annotation task for rework

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { ReturnAnnotationTaskRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies ReturnAnnotationTaskRequest;

  try {
    const data = await api.returnAnnotationTask(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Returned task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## startAnnotationTask

> TaskResponse startAnnotationTask(xTenantId, id, taskId, authorization)

Start an assigned annotation task

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { StartAnnotationTaskRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies StartAnnotationTaskRequest;

  try {
    const data = await api.startAnnotationTask(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Started task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## submitAnnotationTask

> TaskResponse submitAnnotationTask(xTenantId, id, taskId, authorization)

Submit annotations for review

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { SubmitAnnotationTaskRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // string
    taskId: taskId_example,
    // string (optional)
    authorization: authorization_example,
  } satisfies SubmitAnnotationTaskRequest;

  try {
    const data = await api.submitAnnotationTask(body);
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
| **taskId** | `string` |  | [Defaults to `undefined`] |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: `application/json`


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **200** | Submitted task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#api-endpoints) [[Back to Model list]](../README.md#models) [[Back to README]](../README.md)


## uploadPart

> uploadPart(xTenantId, id, partNumber, body, authorization, sig, expires)

Upload a part

### Example

```ts
import {
  Configuration,
  DefaultApi,
} from '';
import type { UploadPartRequest } from '';

async function example() {
  console.log("🚀 Testing  SDK...");
  const api = new DefaultApi();

  const body = {
    // string
    xTenantId: xTenantId_example,
    // string
    id: id_example,
    // number
    partNumber: 56,
    // Blob
    body: BINARY_DATA_HERE,
    // string (optional)
    authorization: authorization_example,
    // string (optional)
    sig: sig_example,
    // number (optional)
    expires: 789,
  } satisfies UploadPartRequest;

  try {
    const data = await api.uploadPart(body);
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
| **partNumber** | `number` |  | [Defaults to `undefined`] |
| **body** | `Blob` |  | |
| **authorization** | `string` |  | [Optional] [Defaults to `undefined`] |
| **sig** | `string` |  | [Optional] [Defaults to `undefined`] |
| **expires** | `number` |  | [Optional] [Defaults to `undefined`] |

### Return type

`void` (Empty response body)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: `application/octet-stream`
- **Accept**: Not defined


### HTTP response details
| Status code | Description | Response headers |
|-------------|-------------|------------------|
| **204** | Part uploaded |  -  |

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

