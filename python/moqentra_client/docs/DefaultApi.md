# moqentra_client.DefaultApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**activate_annotation_project**](DefaultApi.md#activate_annotation_project) | **POST** /v1/annotation-projects/{id}/activate | Activate annotation project
[**admit_training_job**](DefaultApi.md#admit_training_job) | **POST** /v1/training-jobs/{id}/admit | Admit a training job
[**cancel_training_job**](DefaultApi.md#cancel_training_job) | **POST** /v1/training-jobs/{id}/cancel | Cancel a training job
[**compile_application**](DefaultApi.md#compile_application) | **POST** /v1/applications:compile | Compile an application graph
[**create_annotation_project**](DefaultApi.md#create_annotation_project) | **POST** /v1/annotation-projects | Create annotation project
[**create_dataset**](DefaultApi.md#create_dataset) | **POST** /v1/datasets | Create dataset
[**create_dataset_version**](DefaultApi.md#create_dataset_version) | **POST** /v1/dataset-versions | Create dataset version
[**create_experiment**](DefaultApi.md#create_experiment) | **POST** /v1/experiments | Create experiment
[**create_model**](DefaultApi.md#create_model) | **POST** /v1/models | Create model family
[**create_training_job**](DefaultApi.md#create_training_job) | **POST** /v1/training-jobs | Create training job
[**get_dataset**](DefaultApi.md#get_dataset) | **GET** /v1/datasets/{id} | Get dataset by id
[**get_health**](DefaultApi.md#get_health) | **GET** /healthz | Liveness probe
[**get_ready**](DefaultApi.md#get_ready) | **GET** /readyz | Readiness probe
[**list_datasets**](DefaultApi.md#list_datasets) | **GET** /v1/datasets | List datasets for tenant
[**list_experiments**](DefaultApi.md#list_experiments) | **GET** /v1/experiments | List experiments
[**list_models**](DefaultApi.md#list_models) | **GET** /v1/models | List models
[**list_outbox_events**](DefaultApi.md#list_outbox_events) | **GET** /v1/outbox | List outbox events
[**list_training_jobs**](DefaultApi.md#list_training_jobs) | **GET** /v1/training-jobs | List training jobs
[**who_am_i**](DefaultApi.md#who_am_i) | **GET** /v1/whoami | Resolve authenticated principal


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

