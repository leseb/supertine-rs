package main

import (
	"fmt"
	"sort"
	"strings"
)

var text = "However, under extreme overload, the service might not even be able to compute and serve degraded responses. At this point it may have no immediate option but to serve errors. One way to mitigate this scenario is to balance traffic across datacenters such that no datacenter receives more traffic than it has the capacity to process. For example, if a datacenter runs 100 backend tasks and each task can process up to 500 requests per second, the load balancing algorithm will not allow more than 50,000 queries per second to be sent to that datacenter. However, even this constraint can prove insufficient to avoid overload when you're operating at scale. At the end of the day, it's best to build clients and backends to handle resource restrictions gracefully: redirect when possible, serve degraded results when necessary, and handle resource errors transparently when all else fails."

func main() {
	alreadyPrinted := make(map[string]bool)
	textToLower := strings.ToLower(text)
	textWithoutPunctuation := strings.ReplaceAll(textToLower, ".", "")
	textToSlice := strings.Split(textWithoutPunctuation, " ")
	sort.Slice(textToSlice, func(i, j int) bool {
		return textToSlice[i] < textToSlice[j]
	})
	for _, word := range textToSlice {
		if _, ok := alreadyPrinted[word]; !ok {
			// if word
			fmt.Printf("word %q appears %d\n", word, strings.Count(textWithoutPunctuation, word))
			alreadyPrinted[word] = true
		}
	}
}
