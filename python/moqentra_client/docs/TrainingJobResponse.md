# TrainingJobResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**experiment_id** | **str** |  | 
**state** | **str** |  | 

## Example

```python
from moqentra_client.models.training_job_response import TrainingJobResponse

# TODO update the JSON string below
json = "{}"
# create an instance of TrainingJobResponse from a JSON string
training_job_response_instance = TrainingJobResponse.from_json(json)
# print the JSON string representation of the object
print(TrainingJobResponse.to_json())

# convert the object into a dict
training_job_response_dict = training_job_response_instance.to_dict()
# create an instance of TrainingJobResponse from a dict
training_job_response_from_dict = TrainingJobResponse.from_dict(training_job_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


