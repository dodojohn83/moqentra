# moqentra_client.DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**abort_upload_session**](DefaultApi.md#abort_upload_session) | **DELETE** /v1/upload-sessions/{id} | Abort an upload session
[**activate_annotation_project**](DefaultApi.md#activate_annotation_project) | **POST** /v1/annotation-projects/{id}/activate | Activate annotation project
[**add_dataset_version_asset**](DefaultApi.md#add_dataset_version_asset) | **POST** /v1/dataset-versions/{id}/assets | Add an asset to a draft dataset version
[**admit_training_job**](DefaultApi.md#admit_training_job) | **POST** /v1/training-jobs/{id}/admit | Admit a training job
[**approve_annotation_task**](DefaultApi.md#approve_annotation_task) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/approve | Approve an annotation task
[**assign_annotation_task**](DefaultApi.md#assign_annotation_task) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/assign | Assign/claim an annotation task
[**autosave_annotation**](DefaultApi.md#autosave_annotation) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/annotations | Autosave an annotation
[**cancel_import_job**](DefaultApi.md#cancel_import_job) | **DELETE** /v1/import-jobs/{id} | Cancel an import job
[**cancel_training_job**](DefaultApi.md#cancel_training_job) | **POST** /v1/training-jobs/{id}/cancel | Cancel a training job
[**compile_application**](DefaultApi.md#compile_application) | **POST** /v1/applications:compile | Compile an application graph
[**complete_upload_session**](DefaultApi.md#complete_upload_session) | **POST** /v1/upload-sessions/{id}/complete | Complete an upload session
[**create_annotation_project**](DefaultApi.md#create_annotation_project) | **POST** /v1/annotation-projects | Create annotation project
[**create_annotation_tasks**](DefaultApi.md#create_annotation_tasks) | **POST** /v1/annotation-projects/{id}/tasks | Create annotation tasks
[**create_dataset**](DefaultApi.md#create_dataset) | **POST** /v1/datasets | Create dataset
[**create_dataset_version**](DefaultApi.md#create_dataset_version) | **POST** /v1/dataset-versions | Create dataset version
[**create_experiment**](DefaultApi.md#create_experiment) | **POST** /v1/experiments | Create experiment
[**create_import_job**](DefaultApi.md#create_import_job) | **POST** /v1/import-jobs | Create an S3/MinIO import job
[**create_model**](DefaultApi.md#create_model) | **POST** /v1/models | Create model family
[**create_training_job**](DefaultApi.md#create_training_job) | **POST** /v1/training-jobs | Create training job
[**create_upload_session**](DefaultApi.md#create_upload_session) | **POST** /v1/upload-sessions | Create a multipart upload session
[**export_coco**](DefaultApi.md#export_coco) | **GET** /v1/annotation-projects/{id}/export-coco | Export annotations as COCO
[**export_labelu**](DefaultApi.md#export_labelu) | **GET** /v1/annotation-projects/{id}/export-labelu | Export annotations in LabelU native format
[**export_platform**](DefaultApi.md#export_platform) | **GET** /v1/annotation-projects/{id}/export-platform | Export annotations in platform intermediate format
[**generate_dataset_version_splits**](DefaultApi.md#generate_dataset_version_splits) | **POST** /v1/dataset-versions/{id}/splits | Generate deterministic train/val/test splits
[**get_annotation_task**](DefaultApi.md#get_annotation_task) | **GET** /v1/annotation-projects/{id}/tasks/{taskId} | Get annotation task
[**get_asset_media_url**](DefaultApi.md#get_asset_media_url) | **GET** /v1/assets/{assetId}/media-url | Get short-lived signed media URL
[**get_dataset**](DefaultApi.md#get_dataset) | **GET** /v1/datasets/{id} | Get dataset by id
[**get_health**](DefaultApi.md#get_health) | **GET** /healthz | Liveness probe
[**get_import_job**](DefaultApi.md#get_import_job) | **GET** /v1/import-jobs/{id} | Get import job status
[**get_ready**](DefaultApi.md#get_ready) | **GET** /readyz | Readiness probe
[**get_upload_session**](DefaultApi.md#get_upload_session) | **GET** /v1/upload-sessions/{id} | Get upload session
[**import_coco**](DefaultApi.md#import_coco) | **POST** /v1/annotation-projects/{id}/import-coco | Import COCO annotations
[**import_labelu**](DefaultApi.md#import_labelu) | **POST** /v1/annotation-projects/{id}/import-labelu | Import LabelU native annotations
[**import_platform**](DefaultApi.md#import_platform) | **POST** /v1/annotation-projects/{id}/import-platform | Import platform intermediate annotations
[**list_annotation_tasks**](DefaultApi.md#list_annotation_tasks) | **GET** /v1/annotation-projects/{id}/tasks | List annotation tasks
[**list_annotations**](DefaultApi.md#list_annotations) | **GET** /v1/annotation-projects/{id}/tasks/{taskId}/annotations | List annotations for a task
[**list_datasets**](DefaultApi.md#list_datasets) | **GET** /v1/datasets | List datasets for tenant
[**list_experiments**](DefaultApi.md#list_experiments) | **GET** /v1/experiments | List experiments
[**list_models**](DefaultApi.md#list_models) | **GET** /v1/models | List models
[**list_outbox_events**](DefaultApi.md#list_outbox_events) | **GET** /v1/outbox | List outbox events
[**list_part_upload_urls**](DefaultApi.md#list_part_upload_urls) | **GET** /v1/upload-sessions/{id}/part-urls | List signed URLs for uploading parts
[**list_training_jobs**](DefaultApi.md#list_training_jobs) | **GET** /v1/training-jobs | List training jobs
[**list_upload_session_parts**](DefaultApi.md#list_upload_session_parts) | **GET** /v1/upload-sessions/{id}/parts | List upload session parts
[**publish_dataset_version**](DefaultApi.md#publish_dataset_version) | **POST** /v1/dataset-versions/{id}/publish | Publish a dataset version
[**return_annotation_task**](DefaultApi.md#return_annotation_task) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/return | Return an annotation task for rework
[**start_annotation_task**](DefaultApi.md#start_annotation_task) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/start | Start an assigned annotation task
[**submit_annotation_task**](DefaultApi.md#submit_annotation_task) | **POST** /v1/annotation-projects/{id}/tasks/{taskId}/submit | Submit annotations for review
[**upload_part**](DefaultApi.md#upload_part) | **POST** /v1/upload-sessions/{id}/parts/{partNumber} | Upload a part
[**who_am_i**](DefaultApi.md#who_am_i) | **GET** /v1/whoami | Resolve authenticated principal


# **abort_upload_session**
> abort_upload_session(x_tenant_id, id, authorization=authorization)

Abort an upload session

### Example


```python
import moqentra_client
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Abort an upload session
        api_instance.abort_upload_session(x_tenant_id, id, authorization=authorization)
    except Exception as e:
        print("Exception when calling DefaultApi->abort_upload_session: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**204** | Aborted |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **activate_annotation_project**
> AnnotationProjectResponse activate_annotation_project(x_tenant_id, id, authorization=authorization)

Activate annotation project

### Example


```python
import moqentra_client
from moqentra_client.models.annotation_project_response import AnnotationProjectResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Activate annotation project
        api_response = api_instance.activate_annotation_project(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->activate_annotation_project:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->activate_annotation_project: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**AnnotationProjectResponse**](AnnotationProjectResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Activated |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **add_dataset_version_asset**
> DatasetVersionResponse add_dataset_version_asset(x_tenant_id, id, add_asset_request, authorization=authorization, idempotency_key=idempotency_key)

Add an asset to a draft dataset version

### Example


```python
import moqentra_client
from moqentra_client.models.add_asset_request import AddAssetRequest
from moqentra_client.models.dataset_version_response import DatasetVersionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    add_asset_request = moqentra_client.AddAssetRequest() # AddAssetRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Add an asset to a draft dataset version
        api_response = api_instance.add_dataset_version_asset(x_tenant_id, id, add_asset_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->add_dataset_version_asset:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->add_dataset_version_asset: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **add_asset_request** | [**AddAssetRequest**](AddAssetRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**DatasetVersionResponse**](DatasetVersionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Asset added |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **admit_training_job**
> TrainingJobResponse admit_training_job(x_tenant_id, id, authorization=authorization)

Admit a training job

### Example


```python
import moqentra_client
from moqentra_client.models.training_job_response import TrainingJobResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Admit a training job
        api_response = api_instance.admit_training_job(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->admit_training_job:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->admit_training_job: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TrainingJobResponse**](TrainingJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Job admitted |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **approve_annotation_task**
> TaskResponse approve_annotation_task(x_tenant_id, id, task_id, authorization=authorization)

Approve an annotation task

### Example


```python
import moqentra_client
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Approve an annotation task
        api_response = api_instance.approve_annotation_task(x_tenant_id, id, task_id, authorization=authorization)
        print("The response of DefaultApi->approve_annotation_task:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->approve_annotation_task: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Approved task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **assign_annotation_task**
> TaskResponse assign_annotation_task(x_tenant_id, id, task_id, assign_request, authorization=authorization)

Assign/claim an annotation task

### Example


```python
import moqentra_client
from moqentra_client.models.assign_request import AssignRequest
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    assign_request = moqentra_client.AssignRequest() # AssignRequest | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Assign/claim an annotation task
        api_response = api_instance.assign_annotation_task(x_tenant_id, id, task_id, assign_request, authorization=authorization)
        print("The response of DefaultApi->assign_annotation_task:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->assign_annotation_task: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **assign_request** | [**AssignRequest**](AssignRequest.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Assigned task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **autosave_annotation**
> AnnotationResponse autosave_annotation(x_tenant_id, id, task_id, autosave_request, authorization=authorization)

Autosave an annotation

### Example


```python
import moqentra_client
from moqentra_client.models.annotation_response import AnnotationResponse
from moqentra_client.models.autosave_request import AutosaveRequest
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    autosave_request = moqentra_client.AutosaveRequest() # AutosaveRequest | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Autosave an annotation
        api_response = api_instance.autosave_annotation(x_tenant_id, id, task_id, autosave_request, authorization=authorization)
        print("The response of DefaultApi->autosave_annotation:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->autosave_annotation: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **autosave_request** | [**AutosaveRequest**](AutosaveRequest.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**AnnotationResponse**](AnnotationResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Saved annotation |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **cancel_import_job**
> cancel_import_job(x_tenant_id, id, authorization=authorization)

Cancel an import job

### Example


```python
import moqentra_client
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Cancel an import job
        api_instance.cancel_import_job(x_tenant_id, id, authorization=authorization)
    except Exception as e:
        print("Exception when calling DefaultApi->cancel_import_job: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: Not defined

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**204** | Cancelled |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **cancel_training_job**
> TrainingJobResponse cancel_training_job(x_tenant_id, id, authorization=authorization)

Cancel a training job

### Example


```python
import moqentra_client
from moqentra_client.models.training_job_response import TrainingJobResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Cancel a training job
        api_response = api_instance.cancel_training_job(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->cancel_training_job:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->cancel_training_job: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TrainingJobResponse**](TrainingJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Job cancelled |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **compile_application**
> CompileResponse compile_application(x_tenant_id, compile_request, authorization=authorization)

Compile an application graph

### Example


```python
import moqentra_client
from moqentra_client.models.compile_request import CompileRequest
from moqentra_client.models.compile_response import CompileResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    compile_request = moqentra_client.CompileRequest() # CompileRequest | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Compile an application graph
        api_response = api_instance.compile_application(x_tenant_id, compile_request, authorization=authorization)
        print("The response of DefaultApi->compile_application:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->compile_application: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **compile_request** | [**CompileRequest**](CompileRequest.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**CompileResponse**](CompileResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Compiled graph |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **complete_upload_session**
> UploadSessionResponse complete_upload_session(x_tenant_id, id, authorization=authorization)

Complete an upload session

### Example


```python
import moqentra_client
from moqentra_client.models.upload_session_response import UploadSessionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Complete an upload session
        api_response = api_instance.complete_upload_session(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->complete_upload_session:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->complete_upload_session: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Completed |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_annotation_project**
> AnnotationProjectResponse create_annotation_project(x_tenant_id, create_annotation_project_request, authorization=authorization, idempotency_key=idempotency_key)

Create annotation project

### Example


```python
import moqentra_client
from moqentra_client.models.annotation_project_response import AnnotationProjectResponse
from moqentra_client.models.create_annotation_project_request import CreateAnnotationProjectRequest
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_annotation_project_request = moqentra_client.CreateAnnotationProjectRequest() # CreateAnnotationProjectRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create annotation project
        api_response = api_instance.create_annotation_project(x_tenant_id, create_annotation_project_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_annotation_project:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_annotation_project: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_annotation_project_request** | [**CreateAnnotationProjectRequest**](CreateAnnotationProjectRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**AnnotationProjectResponse**](AnnotationProjectResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_annotation_tasks**
> List[TaskResponse] create_annotation_tasks(x_tenant_id, id, create_tasks_request, authorization=authorization)

Create annotation tasks

### Example


```python
import moqentra_client
from moqentra_client.models.create_tasks_request import CreateTasksRequest
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    create_tasks_request = moqentra_client.CreateTasksRequest() # CreateTasksRequest | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Create annotation tasks
        api_response = api_instance.create_annotation_tasks(x_tenant_id, id, create_tasks_request, authorization=authorization)
        print("The response of DefaultApi->create_annotation_tasks:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_annotation_tasks: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **create_tasks_request** | [**CreateTasksRequest**](CreateTasksRequest.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**List[TaskResponse]**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_dataset**
> DatasetResponse create_dataset(x_tenant_id, create_dataset_request, authorization=authorization, idempotency_key=idempotency_key)

Create dataset

### Example


```python
import moqentra_client
from moqentra_client.models.create_dataset_request import CreateDatasetRequest
from moqentra_client.models.dataset_response import DatasetResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_dataset_request = moqentra_client.CreateDatasetRequest() # CreateDatasetRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create dataset
        api_response = api_instance.create_dataset(x_tenant_id, create_dataset_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_dataset:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_dataset: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_dataset_request** | [**CreateDatasetRequest**](CreateDatasetRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**DatasetResponse**](DatasetResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_dataset_version**
> DatasetVersionResponse create_dataset_version(x_tenant_id, create_dataset_version_request, authorization=authorization, idempotency_key=idempotency_key)

Create dataset version

### Example


```python
import moqentra_client
from moqentra_client.models.create_dataset_version_request import CreateDatasetVersionRequest
from moqentra_client.models.dataset_version_response import DatasetVersionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_dataset_version_request = moqentra_client.CreateDatasetVersionRequest() # CreateDatasetVersionRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create dataset version
        api_response = api_instance.create_dataset_version(x_tenant_id, create_dataset_version_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_dataset_version:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_dataset_version: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_dataset_version_request** | [**CreateDatasetVersionRequest**](CreateDatasetVersionRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**DatasetVersionResponse**](DatasetVersionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_experiment**
> ExperimentResponse create_experiment(x_tenant_id, create_experiment_request, authorization=authorization, idempotency_key=idempotency_key)

Create experiment

### Example


```python
import moqentra_client
from moqentra_client.models.create_experiment_request import CreateExperimentRequest
from moqentra_client.models.experiment_response import ExperimentResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_experiment_request = moqentra_client.CreateExperimentRequest() # CreateExperimentRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create experiment
        api_response = api_instance.create_experiment(x_tenant_id, create_experiment_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_experiment:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_experiment: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_experiment_request** | [**CreateExperimentRequest**](CreateExperimentRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**ExperimentResponse**](ExperimentResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_import_job**
> ImportJobResponse create_import_job(x_tenant_id, create_import_job_request, authorization=authorization, idempotency_key=idempotency_key)

Create an S3/MinIO import job

### Example


```python
import moqentra_client
from moqentra_client.models.create_import_job_request import CreateImportJobRequest
from moqentra_client.models.import_job_response import ImportJobResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_import_job_request = moqentra_client.CreateImportJobRequest() # CreateImportJobRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create an S3/MinIO import job
        api_response = api_instance.create_import_job(x_tenant_id, create_import_job_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_import_job:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_import_job: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_import_job_request** | [**CreateImportJobRequest**](CreateImportJobRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**ImportJobResponse**](ImportJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_model**
> ModelResponse create_model(x_tenant_id, create_model_request, authorization=authorization, idempotency_key=idempotency_key)

Create model family

### Example


```python
import moqentra_client
from moqentra_client.models.create_model_request import CreateModelRequest
from moqentra_client.models.model_response import ModelResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_model_request = moqentra_client.CreateModelRequest() # CreateModelRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create model family
        api_response = api_instance.create_model(x_tenant_id, create_model_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_model:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_model: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_model_request** | [**CreateModelRequest**](CreateModelRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**ModelResponse**](ModelResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_training_job**
> TrainingJobResponse create_training_job(x_tenant_id, create_training_job_request, authorization=authorization, idempotency_key=idempotency_key)

Create training job

### Example


```python
import moqentra_client
from moqentra_client.models.create_training_job_request import CreateTrainingJobRequest
from moqentra_client.models.training_job_response import TrainingJobResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_training_job_request = moqentra_client.CreateTrainingJobRequest() # CreateTrainingJobRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create training job
        api_response = api_instance.create_training_job(x_tenant_id, create_training_job_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_training_job:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_training_job: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_training_job_request** | [**CreateTrainingJobRequest**](CreateTrainingJobRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**TrainingJobResponse**](TrainingJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **create_upload_session**
> UploadSessionResponse create_upload_session(x_tenant_id, create_upload_session_request, authorization=authorization, idempotency_key=idempotency_key)

Create a multipart upload session

### Example


```python
import moqentra_client
from moqentra_client.models.create_upload_session_request import CreateUploadSessionRequest
from moqentra_client.models.upload_session_response import UploadSessionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    create_upload_session_request = moqentra_client.CreateUploadSessionRequest() # CreateUploadSessionRequest | 
    authorization = 'authorization_example' # str |  (optional)
    idempotency_key = 'idempotency_key_example' # str |  (optional)

    try:
        # Create a multipart upload session
        api_response = api_instance.create_upload_session(x_tenant_id, create_upload_session_request, authorization=authorization, idempotency_key=idempotency_key)
        print("The response of DefaultApi->create_upload_session:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->create_upload_session: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **create_upload_session_request** | [**CreateUploadSessionRequest**](CreateUploadSessionRequest.md)|  | 
 **authorization** | **str**|  | [optional] 
 **idempotency_key** | **str**|  | [optional] 

### Return type

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Created |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **export_coco**
> CocoDataset export_coco(x_tenant_id, id, authorization=authorization)

Export annotations as COCO

### Example


```python
import moqentra_client
from moqentra_client.models.coco_dataset import CocoDataset
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Export annotations as COCO
        api_response = api_instance.export_coco(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->export_coco:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->export_coco: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**CocoDataset**](CocoDataset.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | COCO dataset |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **export_labelu**
> LabelUDataset export_labelu(x_tenant_id, id, authorization=authorization)

Export annotations in LabelU native format

### Example


```python
import moqentra_client
from moqentra_client.models.label_u_dataset import LabelUDataset
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Export annotations in LabelU native format
        api_response = api_instance.export_labelu(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->export_labelu:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->export_labelu: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**LabelUDataset**](LabelUDataset.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | LabelU native dataset |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **export_platform**
> PlatformAnnotationDataset export_platform(x_tenant_id, id, authorization=authorization)

Export annotations in platform intermediate format

### Example


```python
import moqentra_client
from moqentra_client.models.platform_annotation_dataset import PlatformAnnotationDataset
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Export annotations in platform intermediate format
        api_response = api_instance.export_platform(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->export_platform:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->export_platform: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**PlatformAnnotationDataset**](PlatformAnnotationDataset.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Platform intermediate dataset |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **generate_dataset_version_splits**
> DatasetVersionResponse generate_dataset_version_splits(x_tenant_id, id, generate_splits_request, authorization=authorization)

Generate deterministic train/val/test splits

### Example


```python
import moqentra_client
from moqentra_client.models.dataset_version_response import DatasetVersionResponse
from moqentra_client.models.generate_splits_request import GenerateSplitsRequest
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    generate_splits_request = moqentra_client.GenerateSplitsRequest() # GenerateSplitsRequest | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Generate deterministic train/val/test splits
        api_response = api_instance.generate_dataset_version_splits(x_tenant_id, id, generate_splits_request, authorization=authorization)
        print("The response of DefaultApi->generate_dataset_version_splits:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->generate_dataset_version_splits: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **generate_splits_request** | [**GenerateSplitsRequest**](GenerateSplitsRequest.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**DatasetVersionResponse**](DatasetVersionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Splits generated |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_annotation_task**
> TaskResponse get_annotation_task(x_tenant_id, id, task_id, authorization=authorization)

Get annotation task

### Example


```python
import moqentra_client
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Get annotation task
        api_response = api_instance.get_annotation_task(x_tenant_id, id, task_id, authorization=authorization)
        print("The response of DefaultApi->get_annotation_task:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_annotation_task: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_asset_media_url**
> MediaUrlResponse get_asset_media_url(x_tenant_id, asset_id, authorization=authorization)

Get short-lived signed media URL

### Example


```python
import moqentra_client
from moqentra_client.models.media_url_response import MediaUrlResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    asset_id = 'asset_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Get short-lived signed media URL
        api_response = api_instance.get_asset_media_url(x_tenant_id, asset_id, authorization=authorization)
        print("The response of DefaultApi->get_asset_media_url:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_asset_media_url: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **asset_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**MediaUrlResponse**](MediaUrlResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Signed URL |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_dataset**
> DatasetResponse get_dataset(x_tenant_id, id, authorization=authorization)

Get dataset by id

### Example


```python
import moqentra_client
from moqentra_client.models.dataset_response import DatasetResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Get dataset by id
        api_response = api_instance.get_dataset(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->get_dataset:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_dataset: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**DatasetResponse**](DatasetResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Dataset |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_health**
> HealthResponse get_health()

Liveness probe

### Example


```python
import moqentra_client
from moqentra_client.models.health_response import HealthResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)

    try:
        # Liveness probe
        api_response = api_instance.get_health()
        print("The response of DefaultApi->get_health:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_health: %s\n" % e)
```



### Parameters

This endpoint does not need any parameter.

### Return type

[**HealthResponse**](HealthResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | OK |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_import_job**
> ImportJobResponse get_import_job(x_tenant_id, id, authorization=authorization)

Get import job status

### Example


```python
import moqentra_client
from moqentra_client.models.import_job_response import ImportJobResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Get import job status
        api_response = api_instance.get_import_job(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->get_import_job:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_import_job: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**ImportJobResponse**](ImportJobResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Import job |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_ready**
> ReadyResponse get_ready()

Readiness probe

### Example


```python
import moqentra_client
from moqentra_client.models.ready_response import ReadyResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)

    try:
        # Readiness probe
        api_response = api_instance.get_ready()
        print("The response of DefaultApi->get_ready:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_ready: %s\n" % e)
```



### Parameters

This endpoint does not need any parameter.

### Return type

[**ReadyResponse**](ReadyResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Ready |  -  |
**503** | Not ready |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_upload_session**
> UploadSessionResponse get_upload_session(x_tenant_id, id, authorization=authorization)

Get upload session

### Example


```python
import moqentra_client
from moqentra_client.models.upload_session_response import UploadSessionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Get upload session
        api_response = api_instance.get_upload_session(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->get_upload_session:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->get_upload_session: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Session |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **import_coco**
> List[str] import_coco(x_tenant_id, id, coco_dataset, authorization=authorization)

Import COCO annotations

### Example


```python
import moqentra_client
from moqentra_client.models.coco_dataset import CocoDataset
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    coco_dataset = moqentra_client.CocoDataset() # CocoDataset | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Import COCO annotations
        api_response = api_instance.import_coco(x_tenant_id, id, coco_dataset, authorization=authorization)
        print("The response of DefaultApi->import_coco:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->import_coco: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **coco_dataset** | [**CocoDataset**](CocoDataset.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

**List[str]**

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Imported task ids |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **import_labelu**
> List[str] import_labelu(x_tenant_id, id, label_u_dataset, authorization=authorization)

Import LabelU native annotations

### Example


```python
import moqentra_client
from moqentra_client.models.label_u_dataset import LabelUDataset
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    label_u_dataset = moqentra_client.LabelUDataset() # LabelUDataset | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Import LabelU native annotations
        api_response = api_instance.import_labelu(x_tenant_id, id, label_u_dataset, authorization=authorization)
        print("The response of DefaultApi->import_labelu:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->import_labelu: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **label_u_dataset** | [**LabelUDataset**](LabelUDataset.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

**List[str]**

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Imported task ids |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **import_platform**
> List[str] import_platform(x_tenant_id, id, platform_annotation_dataset, authorization=authorization)

Import platform intermediate annotations

### Example


```python
import moqentra_client
from moqentra_client.models.platform_annotation_dataset import PlatformAnnotationDataset
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    platform_annotation_dataset = moqentra_client.PlatformAnnotationDataset() # PlatformAnnotationDataset | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Import platform intermediate annotations
        api_response = api_instance.import_platform(x_tenant_id, id, platform_annotation_dataset, authorization=authorization)
        print("The response of DefaultApi->import_platform:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->import_platform: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **platform_annotation_dataset** | [**PlatformAnnotationDataset**](PlatformAnnotationDataset.md)|  | 
 **authorization** | **str**|  | [optional] 

### Return type

**List[str]**

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**201** | Imported task ids |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_annotation_tasks**
> List[TaskResponse] list_annotation_tasks(x_tenant_id, id, authorization=authorization)

List annotation tasks

### Example


```python
import moqentra_client
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # List annotation tasks
        api_response = api_instance.list_annotation_tasks(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->list_annotation_tasks:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_annotation_tasks: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**List[TaskResponse]**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Task list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_annotations**
> List[AnnotationResponse] list_annotations(x_tenant_id, id, task_id, authorization=authorization)

List annotations for a task

### Example


```python
import moqentra_client
from moqentra_client.models.annotation_response import AnnotationResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # List annotations for a task
        api_response = api_instance.list_annotations(x_tenant_id, id, task_id, authorization=authorization)
        print("The response of DefaultApi->list_annotations:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_annotations: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**List[AnnotationResponse]**](AnnotationResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Annotation list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_datasets**
> Page list_datasets(x_tenant_id, authorization=authorization, limit=limit, offset=offset)

List datasets for tenant

### Example


```python
import moqentra_client
from moqentra_client.models.page import Page
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)
    limit = 20 # int |  (optional) (default to 20)
    offset = 0 # int |  (optional) (default to 0)

    try:
        # List datasets for tenant
        api_response = api_instance.list_datasets(x_tenant_id, authorization=authorization, limit=limit, offset=offset)
        print("The response of DefaultApi->list_datasets:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_datasets: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 
 **limit** | **int**|  | [optional] [default to 20]
 **offset** | **int**|  | [optional] [default to 0]

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Dataset list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_experiments**
> Page list_experiments(x_tenant_id, authorization=authorization, limit=limit, offset=offset)

List experiments

### Example


```python
import moqentra_client
from moqentra_client.models.page import Page
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)
    limit = 20 # int |  (optional) (default to 20)
    offset = 0 # int |  (optional) (default to 0)

    try:
        # List experiments
        api_response = api_instance.list_experiments(x_tenant_id, authorization=authorization, limit=limit, offset=offset)
        print("The response of DefaultApi->list_experiments:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_experiments: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 
 **limit** | **int**|  | [optional] [default to 20]
 **offset** | **int**|  | [optional] [default to 0]

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Experiment list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_models**
> Page list_models(x_tenant_id, authorization=authorization, limit=limit, offset=offset)

List models

### Example


```python
import moqentra_client
from moqentra_client.models.page import Page
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)
    limit = 20 # int |  (optional) (default to 20)
    offset = 0 # int |  (optional) (default to 0)

    try:
        # List models
        api_response = api_instance.list_models(x_tenant_id, authorization=authorization, limit=limit, offset=offset)
        print("The response of DefaultApi->list_models:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_models: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 
 **limit** | **int**|  | [optional] [default to 20]
 **offset** | **int**|  | [optional] [default to 0]

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Model list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_outbox_events**
> Page list_outbox_events(x_tenant_id, authorization=authorization, limit=limit, offset=offset)

List outbox events

### Example


```python
import moqentra_client
from moqentra_client.models.page import Page
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)
    limit = 20 # int |  (optional) (default to 20)
    offset = 0 # int |  (optional) (default to 0)

    try:
        # List outbox events
        api_response = api_instance.list_outbox_events(x_tenant_id, authorization=authorization, limit=limit, offset=offset)
        print("The response of DefaultApi->list_outbox_events:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_outbox_events: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 
 **limit** | **int**|  | [optional] [default to 20]
 **offset** | **int**|  | [optional] [default to 0]

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Outbox event list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_part_upload_urls**
> List[UploadPartUrl] list_part_upload_urls(x_tenant_id, id, authorization=authorization)

List signed URLs for uploading parts

### Example


```python
import moqentra_client
from moqentra_client.models.upload_part_url import UploadPartUrl
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # List signed URLs for uploading parts
        api_response = api_instance.list_part_upload_urls(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->list_part_upload_urls:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_part_upload_urls: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**List[UploadPartUrl]**](UploadPartUrl.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Signed part upload URLs |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_training_jobs**
> Page list_training_jobs(x_tenant_id, authorization=authorization, limit=limit, offset=offset)

List training jobs

### Example


```python
import moqentra_client
from moqentra_client.models.page import Page
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)
    limit = 20 # int |  (optional) (default to 20)
    offset = 0 # int |  (optional) (default to 0)

    try:
        # List training jobs
        api_response = api_instance.list_training_jobs(x_tenant_id, authorization=authorization, limit=limit, offset=offset)
        print("The response of DefaultApi->list_training_jobs:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_training_jobs: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 
 **limit** | **int**|  | [optional] [default to 20]
 **offset** | **int**|  | [optional] [default to 0]

### Return type

[**Page**](Page.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Job list |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_upload_session_parts**
> UploadSessionResponse list_upload_session_parts(x_tenant_id, id, authorization=authorization)

List upload session parts

### Example


```python
import moqentra_client
from moqentra_client.models.upload_session_response import UploadSessionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # List upload session parts
        api_response = api_instance.list_upload_session_parts(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->list_upload_session_parts:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_upload_session_parts: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**UploadSessionResponse**](UploadSessionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Session |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **publish_dataset_version**
> DatasetVersionResponse publish_dataset_version(x_tenant_id, id, authorization=authorization)

Publish a dataset version

### Example


```python
import moqentra_client
from moqentra_client.models.dataset_version_response import DatasetVersionResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Publish a dataset version
        api_response = api_instance.publish_dataset_version(x_tenant_id, id, authorization=authorization)
        print("The response of DefaultApi->publish_dataset_version:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->publish_dataset_version: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**DatasetVersionResponse**](DatasetVersionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Version published |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **return_annotation_task**
> TaskResponse return_annotation_task(x_tenant_id, id, task_id, authorization=authorization)

Return an annotation task for rework

### Example


```python
import moqentra_client
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Return an annotation task for rework
        api_response = api_instance.return_annotation_task(x_tenant_id, id, task_id, authorization=authorization)
        print("The response of DefaultApi->return_annotation_task:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->return_annotation_task: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Returned task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **start_annotation_task**
> TaskResponse start_annotation_task(x_tenant_id, id, task_id, authorization=authorization)

Start an assigned annotation task

### Example


```python
import moqentra_client
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Start an assigned annotation task
        api_response = api_instance.start_annotation_task(x_tenant_id, id, task_id, authorization=authorization)
        print("The response of DefaultApi->start_annotation_task:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->start_annotation_task: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Started task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **submit_annotation_task**
> TaskResponse submit_annotation_task(x_tenant_id, id, task_id, authorization=authorization)

Submit annotations for review

### Example


```python
import moqentra_client
from moqentra_client.models.task_response import TaskResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    task_id = 'task_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Submit annotations for review
        api_response = api_instance.submit_annotation_task(x_tenant_id, id, task_id, authorization=authorization)
        print("The response of DefaultApi->submit_annotation_task:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->submit_annotation_task: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **task_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**TaskResponse**](TaskResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Submitted task |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **upload_part**
> upload_part(x_tenant_id, id, part_number, body, authorization=authorization, sig=sig, expires=expires)

Upload a part

### Example


```python
import moqentra_client
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    id = 'id_example' # str | 
    part_number = 56 # int | 
    body = None # bytes | 
    authorization = 'authorization_example' # str |  (optional)
    sig = 'sig_example' # str |  (optional)
    expires = 56 # int |  (optional)

    try:
        # Upload a part
        api_instance.upload_part(x_tenant_id, id, part_number, body, authorization=authorization, sig=sig, expires=expires)
    except Exception as e:
        print("Exception when calling DefaultApi->upload_part: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **id** | **str**|  | 
 **part_number** | **int**|  | 
 **body** | **bytes**|  | 
 **authorization** | **str**|  | [optional] 
 **sig** | **str**|  | [optional] 
 **expires** | **int**|  | [optional] 

### Return type

void (empty response body)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: application/octet-stream
 - **Accept**: Not defined

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**204** | Part uploaded |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **who_am_i**
> WhoAmIResponse who_am_i(x_tenant_id, authorization=authorization)

Resolve authenticated principal

### Example


```python
import moqentra_client
from moqentra_client.models.who_am_i_response import WhoAmIResponse
from moqentra_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = moqentra_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with moqentra_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = moqentra_client.DefaultApi(api_client)
    x_tenant_id = 'x_tenant_id_example' # str | 
    authorization = 'authorization_example' # str |  (optional)

    try:
        # Resolve authenticated principal
        api_response = api_instance.who_am_i(x_tenant_id, authorization=authorization)
        print("The response of DefaultApi->who_am_i:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->who_am_i: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **x_tenant_id** | **str**|  | 
 **authorization** | **str**|  | [optional] 

### Return type

[**WhoAmIResponse**](WhoAmIResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Principal context |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

