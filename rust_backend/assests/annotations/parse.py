import csv
import json

# Annotations taken from: https://github.com/openimages/dataset/blob/main/READMEV2.md

def main():
    with open('./class-descriptions.csv', 'r') as f:
        reader = csv.reader(f)
        your_list = list(reader)
        json_out = []
        for entry in your_list:
            json_out.append({"name": entry[1]})

        json_out = json_out[:1000]

        with open('./classes.json', 'w') as f:
            json.dump(json_out, f)



if __name__ == "__main__":
    main()
