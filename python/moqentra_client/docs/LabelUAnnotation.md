# LabelUAnnotation


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**id** | **str** |  | 
**type** | **str** |  | 
**label** | **str** |  | 
**tool** | **str** |  | [optional] 
**frame** | **int** |  | [optional] 
**points** | **List[float]** |  | [optional] 

## Example

```python
from moqentra_client.models.label_u_annotation import LabelUAnnotation

# TODO update the JSON string below
json = "{}"
# create an instance of LabelUAnnotation from a JSON string
label_u_annotation_instance = LabelUAnnotation.from_json(json)
# print the JSON string representation of the object
print(LabelUAnnotation.to_json())

# convert the object into a dict
label_u_annotation_dict = label_u_annotation_instance.to_dict()
# create an instance of LabelUAnnotation from a dict
label_u_annotation_from_dict = LabelUAnnotation.from_dict(label_u_annotation_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


