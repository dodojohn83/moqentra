# ProblemDetailsViolationsInner


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**var_field** | **str** |  | [optional] 
**description** | **str** |  | [optional] 

## Example

```python
from moqentra_client.models.problem_details_violations_inner import ProblemDetailsViolationsInner

# TODO update the JSON string below
json = "{}"
# create an instance of ProblemDetailsViolationsInner from a JSON string
problem_details_violations_inner_instance = ProblemDetailsViolationsInner.from_json(json)
# print the JSON string representation of the object
print(ProblemDetailsViolationsInner.to_json())

# convert the object into a dict
problem_details_violations_inner_dict = problem_details_violations_inner_instance.to_dict()
# create an instance of ProblemDetailsViolationsInner from a dict
problem_details_violations_inner_from_dict = ProblemDetailsViolationsInner.from_dict(problem_details_violations_inner_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


