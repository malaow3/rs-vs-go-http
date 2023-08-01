package main

// import "C".
import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"sync"
	"time"
)

func main() {
	// Get API key from env
	apiKey := getAPIKey()
	start := time.Now()
	// getGames(client, apiKey)
	getTours(apiKey, "vgc23")
	elapsed := time.Since(start)
	fmt.Printf("Time elapsed: %s\n", elapsed)
}

func getAPIKey() string {
	key := os.Getenv("LIMITLESS_API_KEY")
	if key == "" {
		panic("LIMITLESS_API_KEY env variable not set")
	}
	return key
}

//export getTours
func getTours(key string, format string) {
	client := &http.Client{}
	// Get all tournaments
	max_tours_count := ^uint64(0)
	requrl := fmt.Sprintf("https://play.limitlesstcg.com/api/tournaments?format=%s&limit=%d", format, max_tours_count)
	headers := map[string]string{
		"Content-Type": "application/json",
		"X-Access-Key": key,
	}

	req, err := http.NewRequest("GET", requrl, nil)
	if err != nil {
		panic(err)
	}

	for k, v := range headers {
		req.Header.Add(k, v)
	}

	resp, err := client.Do(req)
	if err != nil {
		panic(err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		panic(err)
	}

	jsonOb := []map[string]interface{}{}
	err = json.Unmarshal(body, &jsonOb)
	if err != nil {
		panic(err)
	}
	fmt.Println(len(jsonOb))

	var wg sync.WaitGroup
	entries := [][]map[string]interface{}{}
	for _, tour := range jsonOb {
		wg.Add(1)
		go func(tour map[string]interface{}) {
			defer wg.Done()
			tour_id, ok := tour["id"].(string)
			if !ok {
				panic("Error: tour id is not a string")
			}
			requrl := fmt.Sprintf("https://play.limitlesstcg.com/api/tournaments/%s/standings", tour_id)
			if err != nil {
				panic(err)
			}
			req, err := http.NewRequest("GET", requrl, nil)
			if err != nil {
				panic(err)
			}
			headers := map[string]string{
				"Content-Type": "application/json",
				"X-Access-Key": key,
			}
			for k, v := range headers {
				req.Header.Add(k, v)
			}
			resp, err := client.Do(req)
			if err != nil {
				panic(err)
			}
			defer resp.Body.Close()
			body, err := io.ReadAll(resp.Body)
			if err != nil {
				panic(err)
			}
			// fmt.Println(string(body))
			jsonOb := []map[string]interface{}{}
			err = json.Unmarshal(body, &jsonOb)
			if err != nil {
				panic(err)
			}
			entries = append(entries, jsonOb)
		}(tour)
	}
	wg.Wait()
	fmt.Println(entries)
}

func getGames(client *http.Client, key string) {
	requrl := "https://play.limitlesstcg.com/api/games"
	headers := map[string]string{
		"Content-Type": "application/json",
		"X-Access-Key": key,
	}

	req, err := http.NewRequest("GET", requrl, nil)
	if err != nil {
		panic(err)
	}

	for k, v := range headers {
		req.Header.Add(k, v)
	}

	resp, err := client.Do(req)
	if err != nil {
		panic(err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		panic(err)
	}
	fmt.Println(string(body))
}
