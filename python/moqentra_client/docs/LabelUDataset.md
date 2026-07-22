# LabelUDataset


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**config** | [**LabelUProjectConfig**](LabelUProjectConfig.md) |  | [optional] 
**annotations** | **Dict[str, List[LabelUAnnotation]]** |  | [optional] 

## Example

```python
from moqentra_client.models.label_u_dataset import LabelUDataset

# TODO update the JSON string below
json = "{}"
# create an instance of LabelUDataset from a JSON string
label_u_dataset_instance = LabelUDataset.from_json(json)
# print the JSON string representation of the object
print(LabelUDataset.to_json())

# convert the object into a dict
label_u_dataset_dict = label_u_dataset_instance.to_dict()
# create an instance of LabelUDataset from a dict
label_u_dataset_from_dict = LabelUDataset.from_dict(label_u_dataset_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


